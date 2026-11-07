use super::*;

// Authored for this repository: an 8px red square moves right in a 32px canvas.
const MOVING_SQUARE: &str = r##"{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":32,"h":32,"nm":"Moving red square","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":1,"nm":"Square","sw":8,"sh":8,"sc":"#ff0000","ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},"p":{"a":1,"k":[{"t":0,"s":[0,0,0],"e":[24,0,0],"o":{"x":0.33,"y":0},"i":{"x":0.67,"y":1}},{"t":30,"s":[24,0,0]}]},"a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},"ao":0,"ip":0,"op":60,"st":0,"bm":0}]}"##;

fn pixels(uri: &str) -> image::RgbaImage {
    let encoded = uri.strip_prefix("data:image/png;base64,").unwrap();
    image::load_from_memory(&STANDARD.decode(encoded).unwrap())
        .unwrap()
        .into_rgba8()
}

#[test]
fn a_lottie_square_renders_at_the_requested_time_and_loops_to_its_first_pixels() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let animation =
                StickerAnimation::load("moving-square".into(), MOVING_SQUARE.as_bytes().into(), 32)
                    .await
                    .unwrap();
            let first = pixels(&animation.first_frame);
            let second = pixels(&animation.frame(1000).await.unwrap());
            assert_eq!(first.get_pixel(0, 0).0, [255, 0, 0, 255]);
            assert_eq!(first.get_pixel(24, 0).0, [0, 0, 0, 0]);
            assert_eq!(second.get_pixel(0, 0).0, [0, 0, 0, 0]);
            assert_eq!(second.get_pixel(24, 0).0, [255, 0, 0, 255]);
            assert_eq!(pixels(&animation.frame(2000).await.unwrap()), first);
        });
}

#[test]
fn sticker_assets_cannot_read_local_files_or_fetch_external_resources() {
    for path in [
        "file:///etc/passwd",
        "../private.png",
        "https://example.com/image.png",
    ] {
        let mut json: serde_json::Value = serde_json::from_str(MOVING_SQUARE).unwrap();
        json["assets"] = serde_json::json!([{"id":"image", "p":path, "u":""}]);
        assert!(
            validate(&serde_json::to_vec(&json).unwrap()).is_err(),
            "{path}"
        );
    }
}

#[test]
fn oversized_output_and_excessive_nesting_are_rejected_before_rendering() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(runtime
        .block_on(StickerAnimation::load(
            "oversize".into(),
            MOVING_SQUARE.as_bytes().into(),
            513
        ))
        .is_err());
    let mut json: serde_json::Value = serde_json::from_str(MOVING_SQUARE).unwrap();
    let mut nested = serde_json::Value::Null;
    for _ in 0..70 {
        nested = serde_json::json!([nested]);
    }
    json["extra"] = nested;
    assert!(validate(&serde_json::to_vec(&json).unwrap()).is_err());
}

#[test]
fn recursive_precompositions_are_rejected_before_entering_the_native_decoder() {
    let mut json: serde_json::Value = serde_json::from_str(MOVING_SQUARE).unwrap();
    json["assets"] = serde_json::json!([{"id":"loop", "layers":[{"ty":0,"refId":"loop"}]}]);
    json["layers"] = serde_json::json!([{"ty":0,"refId":"loop"}]);
    assert!(validate(&serde_json::to_vec(&json).unwrap()).is_err());
}

#[test]
fn a_transparent_rectangular_composition_is_centered_without_darkening_its_color() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut json: serde_json::Value = serde_json::from_str(MOVING_SQUARE).unwrap();
            json["h"] = serde_json::json!(16);
            json["layers"][0]["ks"]["o"]["k"] = serde_json::json!(50);
            let animation = StickerAnimation::load(
                "rectangular-alpha".into(),
                serde_json::to_vec(&json).unwrap().into(),
                32,
            )
            .await
            .unwrap();
            let frame = pixels(&animation.first_frame);
            assert_eq!(frame.get_pixel(0, 0).0, [0, 0, 0, 0]);
            let center = frame.get_pixel(0, 8).0;
            assert_eq!(&center[..3], &[255, 0, 0]);
            assert!((127..=128).contains(&center[3]));
        });
}
