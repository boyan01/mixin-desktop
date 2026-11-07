use super::*;

fn run(test: impl std::future::Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(test);
}

#[test]
fn crop_rotate_flip_and_annotation_export_correct_pixels_without_changing_the_source() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("original.png");
        let mut original = RgbaImage::from_pixel(4, 3, Rgba([10, 20, 30, 255]));
        original.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        original.save(&source).unwrap();
        let bytes = std::fs::read(&source).unwrap();
        let edited = apply(
            source.to_string_lossy().into_owned(),
            vec![
                ImageEdit::Stroke {
                    points: vec![(1, 1)],
                    color: [255, 0, 0, 255],
                    width: 1,
                },
                ImageEdit::Crop {
                    x: 1,
                    y: 0,
                    width: 2,
                    height: 3,
                },
                ImageEdit::RotateClockwise,
                ImageEdit::FlipHorizontal,
            ],
        )
        .await
        .unwrap();
        let image = image::open(&edited.path).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (3, 2));
        assert_eq!(*image.get_pixel(0, 0), Rgba([0, 255, 0, 255]));
        assert_eq!(*image.get_pixel(1, 0), Rgba([255, 0, 0, 255]));
        assert_eq!(edited.width, Some(3));
        assert_eq!(edited.height, Some(2));
        assert_eq!(edited.mime_type, "image/png");
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        let path = edited.path.clone();
        let send_guard = edited.clone();
        drop(edited);
        assert!(Path::new(&path).exists());
        drop(send_guard);
        assert!(!Path::new(&path).exists());
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    });
}

#[test]
fn erasing_transformed_annotations_restores_original_pixels_instead_of_making_them_transparent() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        let original = RgbaImage::from_pixel(5, 3, Rgba([25, 50, 75, 128]));
        original.save(&source).unwrap();
        let edited = apply(
            source.to_string_lossy().into_owned(),
            vec![
                ImageEdit::Stroke {
                    points: vec![(1, 1)],
                    color: [255, 0, 0, 255],
                    width: 1,
                },
                ImageEdit::RotateClockwise,
                ImageEdit::EraseStroke {
                    points: vec![(1, 1)],
                    width: 1,
                },
            ],
        )
        .await
        .unwrap();
        let image = image::open(&edited.path).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (3, 5));
        assert_eq!(*image.get_pixel(1, 1), Rgba([25, 50, 75, 128]));
    });
}

#[test]
fn invalid_crop_or_annotation_fails_without_modifying_the_source() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        RgbaImage::from_pixel(3, 3, Rgba([1, 2, 3, 255]))
            .save(&source)
            .unwrap();
        let bytes = std::fs::read(&source).unwrap();
        for edit in [
            ImageEdit::Crop {
                x: u32::MAX,
                y: 0,
                width: 2,
                height: 2,
            },
            ImageEdit::Crop {
                x: 0,
                y: 0,
                width: 0,
                height: 2,
            },
            ImageEdit::Stroke {
                points: vec![(3, 0)],
                color: [255; 4],
                width: 1,
            },
        ] {
            assert!(apply(source.to_string_lossy().into_owned(), vec![edit])
                .await
                .is_err());
            assert_eq!(std::fs::read(&source).unwrap(), bytes);
        }
    });
}

#[test]
fn counterclockwise_rotation_moves_the_original_top_left_pixel_to_the_bottom_left() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.png");
        let mut original = RgbaImage::from_pixel(4, 3, Rgba([0, 0, 0, 255]));
        original.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        original.save(&source).unwrap();
        let edited = apply(
            source.to_string_lossy().into_owned(),
            vec![ImageEdit::RotateCounterClockwise],
        )
        .await
        .unwrap();
        let image = image::open(&edited.path).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (3, 4));
        assert_eq!(*image.get_pixel(0, 3), Rgba([255, 0, 0, 255]));
        assert_eq!(*image.get_pixel(0, 0), Rgba([0, 0, 0, 255]));
    });
}

#[test]
fn a_camera_jpeg_is_oriented_before_crop_coordinates_are_applied() {
    run(async {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("camera.jpg");
        let image = image::RgbImage::from_pixel(4, 2, image::Rgb([80, 120, 160]));
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 95)
            .encode_image(&image)
            .unwrap();
        // An EXIF little-endian IFD with orientation 6 (90 degrees clockwise).
        let exif = [
            b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1,
            0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut bytes = encoded[..2].to_vec();
        bytes.extend([0xff, 0xe1]);
        bytes.extend(((exif.len() + 2) as u16).to_be_bytes());
        bytes.extend(exif);
        bytes.extend(&encoded[2..]);
        std::fs::write(&source, &bytes).unwrap();
        let normalized = apply(source.to_string_lossy().into_owned(), Vec::new())
            .await
            .unwrap();
        assert_eq!((normalized.width, normalized.height), (Some(2), Some(4)));
        let cropped = apply(
            source.to_string_lossy().into_owned(),
            vec![ImageEdit::Crop {
                x: 0,
                y: 2,
                width: 2,
                height: 2,
            }],
        )
        .await
        .unwrap();
        assert_eq!((cropped.width, cropped.height), (Some(2), Some(2)));
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    });
}
