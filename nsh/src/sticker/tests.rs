//! This exercises presentation because renderer-only tests cannot detect a
//! player that never publishes frames to its image view.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use nsh_core::{MountItem, time::Clock};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;
use std::{
    collections::HashSet,
    sync::Mutex,
    time::{Duration, SystemTime},
};

#[component]
fn MovingSticker() -> impl IntoElement {
    provide_context(i18n::I18n::new());
    provide_context(stored(Some(
        Arc::new(mixin_desktop_api::MediaClient::new()),
    )));
    let artwork = framed_asset(None, source, (32.0, 32.0));
    let strip = virtual_list(
        || 1_usize,
        |_| {
            visibility(|| false);
            framed_asset(None, source, (32.0, 32.0))
        },
    )
    .h(px(32.0));
    view().child(artwork).child(strip)
}

fn source() -> (Option<String>, String, String) {
    (
        None,
        format!(
            "data:application/json;base64,{}",
            STANDARD.encode(include_bytes!("../../preview/fixtures/moving-sticker.json"))
        ),
        "json".into(),
    )
}

#[test]
fn an_inline_lottie_sticker_keeps_playing_while_album_thumbnails_are_paused() {
    let mut app = Headless::start(EngineConfig {
        clock: Clock::manual(SystemTime::UNIX_EPOCH),
        ..EngineConfig::new(Root::of::<MovingSticker>())
    })
    .unwrap();
    let frames = Arc::new(Mutex::new(HashSet::new()));
    let observed = frames.clone();
    app.observe(move |_, batch| {
        for item in &batch.items {
            if let MountItem::Image {
                source: Some(source),
                ..
            } = item
                && source.uri.starts_with("data:image/png")
            {
                observed.lock().unwrap().insert(source.uri.clone());
            }
        }
    });
    app.first_render().unwrap();
    app.wait_until(
        "three moving sticker frames",
        Duration::from_secs(3),
        |_| frames.lock().unwrap().len() >= 3,
    )
    .unwrap();
    assert!(
        frames.lock().unwrap().len() >= 3,
        "the sticker did not publish moving frames"
    );
}

#[test]
fn json_stickers_are_recognized_from_the_type_or_source_without_misclassifying_rasters() {
    for (kind, url, expected) in [
        ("JSON", "https://example.com/sticker", true),
        (
            "application/json; charset=utf-8",
            "https://example.com/sticker",
            true,
        ),
        (
            "application/lottie+json",
            "https://example.com/sticker",
            true,
        ),
        (
            "",
            "https://example.com/sticker.JSON?version=2#poster",
            true,
        ),
        ("", "data:application/json;base64,e30=", true),
        ("PNG", "https://example.com/sticker.png", false),
        ("", "https://example.com/sticker.png?name=.json", false),
    ] {
        assert_eq!(is_animation(kind, url), expected, "{kind}: {url}");
    }
}
