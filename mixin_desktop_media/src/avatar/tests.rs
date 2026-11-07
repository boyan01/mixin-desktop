use super::*;

fn run(test: impl std::future::Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(test);
}

#[test]
fn a_selected_square_exports_its_actual_pixels_and_preserves_the_original() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.png");
        let original = image::RgbaImage::from_fn(6, 2, |x, _| {
            if x < 4 {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0, 255, 0, 255])
            }
        });
        original.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let source = prepare(path.to_string_lossy().into_owned()).await.unwrap();
        let leased_path = source.path.clone();
        assert_eq!((source.width, source.height), (Some(6), Some(2)));
        let encoded = encode(
            source.clone(),
            ImageEdit::Crop {
                x: 4,
                y: 0,
                width: 2,
                height: 2,
            },
            false,
        )
        .await
        .unwrap();
        let avatar = image::load_from_memory(&STANDARD.decode(encoded).unwrap())
            .unwrap()
            .to_rgba8();
        assert_eq!(avatar.dimensions(), (2, 2));
        assert!(avatar.pixels().all(|pixel| pixel.0 == [0, 255, 0, 255]));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        drop(source);
        assert!(!Path::new(&leased_path).exists());
    });
}

#[test]
fn avatar_export_rejects_non_square_zero_or_outside_crops_and_other_edits() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.png");
        image::RgbaImage::new(4, 3).save(&path).unwrap();
        let source = prepare(path.to_string_lossy().into_owned()).await.unwrap();
        for edit in [
            ImageEdit::Crop {
                x: 0,
                y: 0,
                width: 4,
                height: 3,
            },
            ImageEdit::Crop {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            ImageEdit::Crop {
                x: u32::MAX,
                y: 0,
                width: 2,
                height: 2,
            },
            ImageEdit::RotateClockwise,
        ] {
            assert!(encode(source.clone(), edit, false).await.is_err());
        }
    });
}

#[test]
fn a_large_square_exports_no_more_than_512_pixels_per_side() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.png");
        image::RgbImage::new(640, 640).save(&path).unwrap();
        let source = prepare(path.to_string_lossy().into_owned()).await.unwrap();
        let encoded = encode(
            source,
            ImageEdit::Crop {
                x: 0,
                y: 0,
                width: 640,
                height: 640,
            },
            false,
        )
        .await
        .unwrap();
        let avatar = image::load_from_memory(&STANDARD.decode(encoded).unwrap()).unwrap();
        assert_eq!((avatar.width(), avatar.height()), (512, 512));
    });
}

#[test]
fn a_camera_jpeg_is_normalized_before_the_preview_and_crop_share_coordinates() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("camera.jpg");
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut encoded)
            .encode_image(&image::RgbImage::new(6, 2))
            .unwrap();
        let exif = [
            b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1,
            0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut bytes = encoded[..2].to_vec();
        bytes.extend([0xff, 0xe1]);
        bytes.extend(((exif.len() + 2) as u16).to_be_bytes());
        bytes.extend(exif);
        bytes.extend(&encoded[2..]);
        std::fs::write(&path, &bytes).unwrap();
        let source = prepare(path.to_string_lossy().into_owned()).await.unwrap();
        assert_eq!((source.width, source.height), (Some(2), Some(6)));
        let preview = image::open(&source.path).unwrap();
        assert_eq!((preview.width(), preview.height()), (2, 6));
        assert!(encode(
            source,
            ImageEdit::Crop {
                x: 0,
                y: 4,
                width: 2,
                height: 2
            },
            false
        )
        .await
        .is_ok());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    });
}

#[test]
fn oversized_or_disguised_sources_fail_before_preparation() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("image.png");
        std::fs::File::create(&path)
            .unwrap()
            .set_len(SOURCE_BYTES + 1)
            .unwrap();
        assert!(prepare(path.to_string_lossy().into_owned()).await.is_err());
        std::fs::write(&path, b"not an image").unwrap();
        assert!(prepare(path.to_string_lossy().into_owned()).await.is_err());
    });
}

#[test]
fn group_exports_a_jpeg_using_the_unpadded_url_safe_server_encoding() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rgba.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 80, 255]))
            .save(&path)
            .unwrap();
        let source = prepare(path.to_string_lossy().into_owned()).await.unwrap();
        let encoded = encode(
            source,
            ImageEdit::Crop {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            true,
        )
        .await
        .unwrap();
        assert!(!encoded.contains(['=', '+', '/']));
        let bytes = URL_SAFE_NO_PAD.decode(encoded).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Jpeg);
        let avatar = image::load_from_memory(&bytes).unwrap();
        assert_eq!((avatar.width(), avatar.height()), (2, 2));
    });
}

#[test]
fn all_supported_source_formats_prepare_as_static_pngs() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        for (format, extension) in [
            (ImageFormat::Png, "png"),
            (ImageFormat::Jpeg, "jpg"),
            (ImageFormat::Gif, "gif"),
            (ImageFormat::WebP, "webp"),
            (ImageFormat::Bmp, "bmp"),
        ] {
            let path = directory.path().join(format!("source.{extension}"));
            image::RgbImage::new(3, 2)
                .save_with_format(&path, format)
                .unwrap();
            let source = prepare(path.to_string_lossy().into_owned()).await.unwrap();
            assert_eq!(
                (source.width, source.height),
                (Some(3), Some(2)),
                "{extension}"
            );
            assert_eq!(
                image::guess_format(&std::fs::read(&source.path).unwrap()).unwrap(),
                ImageFormat::Png
            );
        }
    });
}
