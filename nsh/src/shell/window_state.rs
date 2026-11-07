//! Native shell presentation and process-wide window preferences.
use super::Handle;
use nsh_core::prelude::*;
use serde_json::{Value, json};

#[cfg(target_os = "windows")]
const BOUNDS_KEY: &str = "desktop_window_bounds_windows";
#[cfg(target_os = "linux")]
const BOUNDS_KEY: &str = "desktop_window_bounds_linux";

pub(super) fn surface(handle: Handle) -> impl IntoElement {
    let ready = signal(false);
    let initial = signal(Value::Null);
    let quitting = signal(false);
    let exit_sent = signal(false);
    let (writer, mut updates) = tokio::sync::watch::channel(None::<(Option<String>, bool)>);
    let writer = stored(writer);
    spawn(async move {
        let result = crate::runtime::run(async {
            let settings = mixin_desktop_api::DesktopClient::open().await?.settings();
            let bounds = settings.setting(BOUNDS_KEY.into()).await?;
            Ok((settings, bounds))
        })
        .await;
        let settings = match result {
            Ok((settings, bounds)) => {
                initial.set(
                    bounds
                        .as_deref()
                        .and_then(parse_bounds)
                        .unwrap_or(Value::Null),
                );
                Some(settings)
            }
            Err(error) => {
                log::warn!("Could not load window bounds: {error}");
                None
            }
        };
        ready.set(true);
        while updates.changed().await.is_ok() {
            let Some((value, quit)) = updates.borrow_and_update().clone() else {
                continue;
            };
            if let (Some(settings), Some(value)) = (settings.clone(), value)
                && let Err(error) = crate::runtime::commit(async move {
                    settings.set_setting(BOUNDS_KEY.into(), Some(value)).await
                })
                .await
            {
                log::warn!("Could not save window bounds: {error}");
            }
            if quit {
                // Keep the engine alive until its last queued geometry write
                // completes. Native exit alone cannot deliver a queued event.
                if !exit_sent.get_untracked() {
                    exit_sent.set(true);
                    handle.reference.command("quit", Value::Null);
                }
                return;
            }
        }
    });
    // Mount after the preference read. Sending an early null would consume the
    // native host's one-time restore before the saved position is available.
    when(
        move || ready.get(),
        move || {
            native("MixinShell")
                .element_ref(&handle.reference)
                .prop("bounds", move || initial.get())
                .prop("labels", move || {
                    json!({
                        "show": crate::i18n::tr("Show Mixin", "显示 Mixin"),
                        "quit": crate::i18n::tr("Quit Mixin", "退出 Mixin"),
                    })
                })
                .prop("tooltip", move || {
                    let count = handle.badge.get();
                    if count > 0 {
                        format!(
                            "Mixin Messenger · {count} {}",
                            crate::i18n::current().text(crate::i18n::Key::Unread)
                        )
                    } else {
                        "Mixin Messenger".into()
                    }
                })
                .on_native_event(move |event: &NativeEvent| {
                    if quitting.get_untracked() {
                        return;
                    }
                    let quit = event.name.as_ref() == "quit";
                    if quit || event.name.as_ref() == "bounds" {
                        let bounds = parse_bounds(&event.data.to_string()).map(|v| v.to_string());
                        if !quit && bounds.is_none() {
                            return;
                        }
                        quitting.set(quit);
                        if quit {
                            spawn(async move {
                                // A stuck earlier settings write must not hold
                                // the native event loop open indefinitely.
                                timer(std::time::Duration::from_secs(2)).await;
                                if !exit_sent.get_untracked() {
                                    exit_sent.set(true);
                                    handle.reference.command("quit", Value::Null);
                                }
                            });
                        }
                        writer.with(|sender| {
                            let _ = sender.send(Some((bounds, quit)));
                        });
                    }
                })
                .w_0()
                .h_0()
        },
    )
    .into_element()
}

fn parse_bounds(raw: &str) -> Option<Value> {
    let value: Value = serde_json::from_str(raw).ok()?;
    #[cfg(target_os = "windows")]
    let fields = ["x", "y", "width", "height"].as_slice();
    #[cfg(target_os = "linux")]
    let fields = ["width", "height"].as_slice();
    for &key in fields {
        let number = i32::try_from(value.get(key)?.as_i64()?).ok()?;
        if matches!(key, "width" | "height") && number <= 0 {
            return None;
        }
    }
    value.get("maximized")?.as_bool()?;
    Some(value)
}
