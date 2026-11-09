//! Native pointer input for the image canvas; it owns no image or zoom state.
use nsh_core::prelude::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::register;

pub(super) fn surface(
    enabled: impl Fn() -> bool + Copy + 'static,
    zoom: impl Fn(f32, (f32, f32)) + Copy + 'static,
    pan: impl Fn((f32, f32)) + Copy + 'static,
    toggle: impl Fn((f32, f32)) + Copy + 'static,
) -> impl IntoElement {
    #[cfg(target_os = "macos")]
    {
        native("MixinImageInput")
            .absolute()
            .inset_0()
            .prop("enabled", enabled)
            .on_native_event(move |event: &NativeEvent| {
                if !enabled() {
                    return;
                }
                let number = |key| {
                    event
                        .data
                        .get(key)
                        .and_then(serde_json::Value::as_f64)
                        .map(|v| v as f32)
                };
                match event.name.as_ref() {
                    "zoom" => {
                        if let (Some(factor), Some(x), Some(y)) =
                            (number("factor"), number("x"), number("y"))
                        {
                            zoom(factor, (x, y));
                        }
                    }
                    "pan" => {
                        if let (Some(x), Some(y)) = (number("x"), number("y")) {
                            pan((x, y));
                        }
                    }
                    "toggle" => {
                        if let (Some(x), Some(y)) = (number("x"), number("y")) {
                            toggle((x, y));
                        }
                    }
                    _ => {}
                }
            })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let drag = stored(None::<(f32, f32)>);
        let _ = (zoom, toggle);
        view()
            .absolute()
            .inset_0()
            .claim_touches(TouchClaim::on_start().ask(move |request: &ClaimRequest| {
                if enabled() {
                    request.claim();
                }
            }))
            .keep_touches(true)
            .on_responder_grant(move |event: &ResponderEvent| {
                drag.replace(Some((event.page.x.0, event.page.y.0)));
            })
            .on_responder_move(move |event: &ResponderEvent| {
                let next = (event.page.x.0, event.page.y.0);
                if let Some(previous) = drag.replace(Some(next)) {
                    pan((next.0 - previous.0, next.1 - previous.1));
                }
            })
            .on_responder_release(move |_: &ResponderEvent| {
                drag.replace(None);
            })
            .on_responder_terminate(move |_: &ResponderEvent| {
                drag.replace(None);
            })
    }
}
