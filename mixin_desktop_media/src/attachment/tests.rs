use super::*;

#[test]
fn prepared_compressed_image_keeps_its_file_alive_until_the_last_send_handle_drops() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("image.png");
    image::RgbaImage::from_pixel(2400, 1200, image::Rgba([10, 20, 30, 80]))
        .save(&path)
        .unwrap();
    let prepared = prepare_file(&path, true, true).unwrap();
    assert_eq!((prepared.width, prepared.height), (Some(2048), Some(1024)));
    assert_eq!(prepared.mime_type, "image/png");
    assert!(prepared.thumbnail.is_some());
    let send_handle = prepared.clone();
    let output = PathBuf::from(&send_handle.path);
    drop(prepared);
    assert!(output.is_file());
    assert_eq!(
        image::open(&output).unwrap().to_rgba8().get_pixel(0, 0).0[3],
        80
    );
    drop(send_handle);
    assert!(!output.exists());
    assert!(path.exists());
}

#[test]
fn video_metadata_applies_rotation_and_rejects_missing_or_nonpositive_duration() {
    let info = parse_video_info(br#"{"streams":[{"width":1920,"height":1080,"side_data_list":[{"rotation":-90}]}],"format":{"duration":"1.25"}}"#).unwrap();
    assert_eq!(info, (1080, 1920, 1250));
    for duration in ["0", "-1", "NaN", "inf"] {
        let value = serde_json::json!({"streams":[{"width":1920,"height":1080}],"format":{"duration":duration}});
        assert!(parse_video_info(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    assert!(parse_video_info(br#"{"streams":[]}"#).is_err());
}

#[test]
fn camera_orientation_changes_display_metadata_and_compressed_pixels_without_touching_the_original()
{
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("camera.jpg");
    let original = image::RgbImage::from_fn(40, 20, |x, _| {
        if x < 20 {
            image::Rgb([240, 0, 0])
        } else {
            image::Rgb([0, 0, 240])
        }
    });
    let mut encoded = Vec::new();
    JpegEncoder::new_with_quality(&mut encoded, 95)
        .encode_image(&original)
        .unwrap();
    let exif = [
        b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1, 0,
        0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut bytes = encoded[..2].to_vec();
    bytes.extend([0xff, 0xe1]);
    bytes.extend(((exif.len() + 2) as u16).to_be_bytes());
    bytes.extend(exif);
    bytes.extend(&encoded[2..]);
    std::fs::write(&source, &bytes).unwrap();
    for compress in [false, true] {
        let item = prepare_file(&source, true, compress).unwrap();
        assert_eq!((item.width, item.height), (Some(20), Some(40)));
        let thumb =
            image::load_from_memory(&STANDARD.decode(item.thumbnail.as_deref().unwrap()).unwrap())
                .unwrap()
                .to_rgb8();
        assert_eq!(thumb.dimensions(), (20, 40));
        assert!(thumb.get_pixel(10, 5)[0] > 200 && thumb.get_pixel(10, 35)[2] > 200);
        if compress {
            let output = image::open(&item.path).unwrap().to_rgb8();
            assert_eq!(output.dimensions(), (20, 40));
            assert!(output.get_pixel(10, 5)[0] > 200 && output.get_pixel(10, 35)[2] > 200);
        } else {
            assert_eq!(Path::new(&item.path), source.canonicalize().unwrap());
            assert_eq!(std::fs::read(&item.path).unwrap(), bytes);
        }
    }
    assert_eq!(std::fs::read(source).unwrap(), bytes);
}

#[test]
fn preparing_an_animated_gif_preserves_all_original_frames_even_with_compression_enabled() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("animated.gif");
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
            encoder
                .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                    12,
                    8,
                    image::Rgba(color),
                )))
                .unwrap();
        }
    }
    std::fs::write(&source, &bytes).unwrap();
    let item = prepare_file(&source, true, true).unwrap();
    assert_eq!(item.mime_type, "image/gif");
    assert_eq!((item.width, item.height), (Some(12), Some(8)));
    assert_eq!(Path::new(&item.path), source.canonicalize().unwrap());
    assert_eq!(std::fs::read(item.path).unwrap(), bytes);
    use image::AnimationDecoder as _;
    let frames = image::codecs::gif::GifDecoder::new(Cursor::new(bytes))
        .unwrap()
        .into_frames()
        .collect_frames()
        .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].buffer().get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(frames[1].buffer().get_pixel(0, 0).0, [0, 0, 255, 255]);
}

#[cfg(target_os = "macos")]
#[test]
fn native_video_preparation_applies_container_rotation_and_exports_a_matching_jpeg_poster() {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("rotated.mp4");
        let bytes = include_bytes!("fixtures/rotated_red_blue.mp4");
        std::fs::write(&source, bytes).unwrap();
        let prepared = prepare(source.to_string_lossy().into_owned(), true, false)
            .await
            .unwrap();
        assert_eq!(prepared.kind, "VIDEO");
        assert_eq!((prepared.width, prepared.height), (Some(90), Some(160)));
        assert!(prepared.duration_millis.unwrap().abs_diff(1000) <= 1);
        let poster = STANDARD
            .decode(prepared.thumbnail.as_deref().unwrap())
            .unwrap();
        assert_eq!(image::guess_format(&poster).unwrap(), ImageFormat::Jpeg);
        let poster = image::load_from_memory(&poster).unwrap().to_rgb8();
        assert_eq!(poster.dimensions(), (90, 160));
        let top = poster.get_pixel(45, 20);
        let bottom = poster.get_pixel(45, 140);
        assert!(top[2] > 200 && top[0] < 30);
        assert!(bottom[0] > 200 && bottom[2] < 30);
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    });
}

#[cfg(target_os = "macos")]
#[test]
fn native_video_preparation_rejects_corrupt_media_without_returning_sendable_metadata() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("invalid.mp4");
        std::fs::write(&source, b"this is not a video").unwrap();
        assert!(prepare(source.to_string_lossy().into_owned(), true, false)
            .await
            .is_err());
        let as_file = prepare(source.to_string_lossy().into_owned(), false, false)
            .await
            .unwrap();
        assert_eq!(as_file.kind, "DATA");
        assert!(as_file.duration_millis.is_none() && as_file.thumbnail.is_none());
        assert_eq!(std::fs::read(source).unwrap(), b"this is not a video");
    });
}

#[test]
fn pasted_files_keep_original_bytes_and_safe_names_until_last_handle_drops() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let bytes = b"NSH migration acceptance\0binary file".to_vec();
            let item = prepare_clipboard_file("../../notes.txt".into(), bytes.clone(), true, false)
                .await
                .unwrap();
            let path = PathBuf::from(&item.path);
            assert_eq!(item.name, "notes.txt");
            assert_eq!(item.kind, "DATA");
            assert_eq!(item.mime_type, "text/plain");
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            let retained = item.clone();
            drop(item);
            assert!(path.is_file());
            drop(retained);
            assert!(!path.exists());
            assert!(
                prepare_clipboard_file("empty.txt".into(), vec![], false, false)
                    .await
                    .is_err()
            );
        });
}

#[test]
fn pasted_camera_images_use_same_preparation_and_keep_compressed_export_alive() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let image = image::RgbaImage::from_pixel(20, 10, image::Rgba([10, 20, 30, 80]));
            let mut encoded = Cursor::new(Vec::new());
            image.write_to(&mut encoded, ImageFormat::Png).unwrap();
            let item =
                prepare_clipboard_file("copied.png".into(), encoded.into_inner(), true, true)
                    .await
                    .unwrap();
            assert_eq!(item.name, "copied.png");
            assert_eq!(item.kind, "IMAGE");
            assert_eq!((item.width, item.height), (Some(20), Some(10)));
            let path = PathBuf::from(&item.path);
            assert_eq!(
                image::open(&path).unwrap().to_rgba8().get_pixel(0, 0).0,
                [10, 20, 30, 80]
            );
            drop(item);
            assert!(!path.exists());
        });
}
