//! Sticker presentation. JSON decoding and raster caches belong to shared media.

use std::{rc::Rc, sync::Arc};

use mixin_desktop_api::{AccountClient, StickerAnimationClient};
use nsh_core::{platform::Lifecycle, prelude::*};

use crate::{
    i18n::{self, Key},
    runtime, ui,
};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
struct Visible(Memo<bool>);

/// A virtual list supplies its real viewability, including recycled cells.
pub(crate) fn visibility(visible: impl Fn() -> bool + 'static) {
    provide_context(Visible(computed(visible)));
}

pub(crate) fn asset(
    account: Arc<AccountClient>,
    source: impl Fn() -> (Option<String>, String, String) + 'static,
    size: f32,
) -> impl IntoElement {
    framed_asset(Some(account), source, (size, size))
}

/// A sticker in a frame of the caller's `(width, height)`. Without an account, as
/// in the debug preview, raster and inline JSON assets work without a refresh service.
pub(crate) fn framed_asset(
    account: Option<Arc<AccountClient>>,
    source: impl Fn() -> (Option<String>, String, String) + 'static,
    frame: (f32, f32),
) -> impl IntoElement {
    let i18n = i18n::current();
    // The caller's bitmap size: physical geometry, not a design token.
    let (width, height) = (px(frame.0), px(frame.1));
    let account = stored(account);
    let source = computed(source);
    let requested = stored(None::<(String, String)>);
    dynamic(move || {
        let expected = source.get();
        untracked(|| {
            let (_, url, kind) = expected.clone();
            let failed = signal(false);
            let fail: Rc<dyn Fn()> = Rc::new(move || {
                // Native errors belong to the URL and identity actually loaded,
                // never to a replacement item occupying the same virtual cell.
                if source.get_untracked() != expected {
                    return;
                }
                failed.set(true);
                let Some(id) = expected.0.as_ref().filter(|id| !id.is_empty()) else {
                    return;
                };
                let key = (id.clone(), expected.1.clone());
                if requested.with(|value| value.as_ref() == Some(&key)) {
                    return;
                }
                let Some(account) = account.with(|value| value.clone()) else {
                    return;
                };
                requested.replace(Some(key));
                let id = id.clone();
                spawn(async move {
                    // The durable job may finish after this view is disposed.
                    let _ =
                        runtime::commit(async move { account.sticker().refresh_sticker(id).await })
                            .await;
                });
            });
            let client = account.with(|value| value.clone());
            let animation = is_animation(&kind, &url);
            let child =
                if url.is_empty() || (animation && client.is_none() && !url.starts_with("data:")) {
                    let fail = fail.clone();
                    effect(move || fail());
                    view().hidden().into_element()
                } else if animation {
                    animated(client, url, frame, failed, fail).into_element()
                } else {
                    let fail = fail.clone();
                    image(url)
                        .w(width)
                        .h(height)
                        .object_fit(ResizeMode::Contain)
                        .label(move || i18n.text(Key::Sticker))
                        .on_error(move |_: &str| fail())
                        .into_element()
                };
            view()
                .w(width)
                .h(height)
                .relative()
                .child(child)
                .child(when(failed, move || {
                    view()
                        .absolute()
                        .inset_0()
                        .items_center()
                        .justify_center()
                        .child(
                            ui::icon(ui::Icon::Sticker)
                                .size(ui::icon_lg())
                                .label(move || i18n.text(Key::CommandFailed)),
                        )
                }))
        })
    })
}

pub(crate) fn framed_album_icon(
    account: Option<Arc<AccountClient>>,
    url: impl Fn() -> String + 'static,
    size: f32,
) -> impl IntoElement {
    framed_asset(
        account,
        move || {
            let url = url();
            let json = url
                .split('?')
                .next()
                .is_some_and(|path| path.to_ascii_lowercase().ends_with(".json"));
            (None, url, if json { "json".into() } else { String::new() })
        },
        (size, size),
    )
}

fn animated(
    account: Option<Arc<AccountClient>>,
    url: String,
    size: (f32, f32),
    failed: Signal<bool>,
    fail: Rc<dyn Fn()>,
) -> impl IntoElement {
    let i18n = i18n::current();
    let security = use_context::<crate::security::Handle>();
    let visible = use_context::<Visible>();
    let media = crate::media_client();
    let animation = signal_with(None::<StickerAnimationClient>, SignalEquals::Never);
    let frame = signal(SharedString::default());
    // The component/virtual cell owner cancels both network work and playback.
    let load_fail = fail.clone();
    effect(move || {
        let account = account.clone();
        let url = url.clone();
        let fail = load_fail.clone();
        let media = media.clone();
        let pixels = (size.0.max(size.1) * pixel_ratio())
            .ceil()
            .clamp(1.0, 512.0) as u32;
        spawn(async move {
            match runtime::run(async move {
                if url.starts_with("data:") {
                    media.sticker_animation_data(url, pixels).await
                } else if let Some(account) = account {
                    account.sticker_animation(url, pixels).await
                } else {
                    Err(mixin_desktop_api::ClientError::Cancelled)
                }
            })
            .await
            {
                Ok(value) => animation.set(Some(value)),
                Err(_) => fail(),
            }
        });
    });
    let render_fail = fail.clone();
    let playing = computed(move || {
        !failed.get()
            && !reduce_motion()
            && lifecycle() != Lifecycle::Background
            && visible.is_none_or(|visible| visible.0.get())
            && !security.is_some_and(|security| security.blocked())
    });
    effect(move || {
        let loaded = animation.get();
        let playing = playing.get();
        let Some(animation) = loaded else { return };
        frame.set(animation.first_frame().into());
        if !playing {
            return;
        }
        let fail = render_fail.clone();
        spawn(async move {
            let start = nsh_core::time::now();
            loop {
                // Stickers are authored at 30 fps or less; a faster clock only
                // repeats frames through the one shared render worker.
                timer(nsh_core::time::FRAME * 2).await;
                let elapsed = nsh_core::time::now().duration_since(start).as_millis() as u64;
                let render = animation.clone();
                // One awaited request per owner: slow frames are skipped using
                // elapsed time rather than building a queue of obsolete frames.
                match runtime::run(async move { render.frame(elapsed).await }).await {
                    Ok(value) => frame.set(value.into()),
                    Err(_) => {
                        fail();
                        break;
                    }
                }
            }
        });
    });
    when(
        move || !frame.get().is_empty(),
        move || {
            let fail = fail.clone();
            image(move || frame.get())
                // The caller's bitmap size: physical geometry, not a design token.
                .w(px(size.0))
                .h(px(size.1))
                .object_fit(ResizeMode::Contain)
                .label(move || i18n.text(Key::Sticker))
                .on_error(move |_: &str| {
                    if !frame.get_untracked().is_empty() {
                        fail();
                    }
                })
        },
    )
}

fn is_animation(kind: &str, url: &str) -> bool {
    let kind = kind.split(';').next().unwrap_or_default().trim();
    kind.eq_ignore_ascii_case("json")
        || kind.eq_ignore_ascii_case("application/json")
        || kind.eq_ignore_ascii_case("application/lottie+json")
        || url.starts_with("data:application/json;")
        || url.starts_with("data:application/lottie+json;")
        || url
            .split(['?', '#'])
            .next()
            .is_some_and(|path| path.to_ascii_lowercase().ends_with(".json"))
}
