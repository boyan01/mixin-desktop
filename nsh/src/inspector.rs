//! Owns the right-hand destination; feature pages own their own state and queries.

mod pinned;
mod search;

use std::sync::Arc;

use mixin_desktop_api::AccountClient;
use nsh_core::prelude::*;

use crate::{conversation, navigation::Panel, ui};

pub(crate) fn screen(
    account: Arc<AccountClient>,
    id: String,
    destination: Panel,
    close: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
) -> impl IntoElement {
    let panel = view().flex_1().min_w_0().min_h_0();
    if destination == Panel::Info {
        return panel
            .child(conversation::screen(account, id, target, close))
            .into_element();
    }
    if destination == Panel::Search {
        return panel
            .child(search::screen(account, id, close, target))
            .into_element();
    }
    panel
        .child(pinned::screen(account, id, close, target))
        .into_element()
}

#[cfg(debug_assertions)]
pub(crate) use {pinned::preview as preview_pinned, search::preview as preview_search};

/// An inspector over the chat toolbar. Outside presses dismiss and are consumed;
/// the timeline keeps its full width throughout opening and closing.
pub(crate) fn overlay(
    destination: Signal<Option<Panel>>,
    content: impl Fn(Panel) -> AnyElement + Copy + 'static,
) -> View {
    let progress = crate::motion::Tween::new(if destination.get_untracked().is_some() {
        1.0
    } else {
        0.0
    });
    effect(move || {
        let open = destination.get().is_some();
        untracked(|| {
            progress.to(
                if open { 1.0 } else { 0.0 },
                crate::motion::PANEL_WIDTH.duration_ms,
            )
        });
    });
    let shown = signal(destination.get_untracked());
    effect(move || {
        if let Some(mode) = destination.get() {
            shown.set(Some(mode));
        }
    });
    view()
        .absolute()
        .inset_0()
        .p_2()
        .items_end()
        .overflow_hidden()
        .pointer_events(PointerEvents::BoxNone)
        .child(when(
            move || destination.get().is_some(),
            move || {
                view()
                    .absolute()
                    .inset_0()
                    .label("Dismiss chat panel")
                    .on_press(move || destination.set(None))
            },
        ))
        .child(when(
            move || destination.get().is_some() || progress.get() > 0.0,
            move || {
                let panel = element_ref();
                let focused = stored(false);
                effect(move || {
                    if progress.get() >= 1.0
                        && destination.get().is_some()
                        && destination.get() != Some(Panel::Search)
                        && !focused.replace(true)
                    {
                        request_animation_frame(move |_| {
                            if destination.get_untracked().is_some()
                                && destination.get_untracked() != Some(Panel::Search)
                            {
                                panel.focus();
                            }
                        });
                    }
                });
                ui::glass(view())
                    .element_ref(&panel)
                    .focusable(true)
                    .label("Chat panel")
                    .on_press(|| {})
                    .w(ui::sidebar_width)
                    .max_w(relative(1.0))
                    .flex_1()
                    .min_h_0()
                    .rounded(ui::radius_lg())
                    .overflow_hidden()
                    .translate_x(move || {
                        px((ui::sidebar_width().0 + ui::space_sm().0) * (1.0 - progress.get()))
                    })
                    .accessibility_elements_hidden(move || destination.get().is_none())
                    .pointer_events(move || {
                        if destination.get().is_some() {
                            PointerEvents::Auto
                        } else {
                            PointerEvents::None
                        }
                    })
                    .child(dynamic(move || {
                        let mode = shown.get();
                        untracked(|| mode.map(content).unwrap_or_else(|| view().into_element()))
                    }))
            },
        ))
}
