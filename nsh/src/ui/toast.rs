//! A transient confirmation or failure notice, shown over the window. It does not take
//! input, so it never blocks the page beneath, and a newer notice replaces the shown one.

use std::time::Duration;

use nsh_core::{element::announce_for_accessibility, prelude::*};

use super::{
    icon::{Icon, icon},
    tokens::{
        TextRole, Typography, border, danger, fg_primary, icon_md, radius_lg, space_md, space_sm,
        space_xl, success,
    },
};
use crate::motion::Presence;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ToastKind {
    Success,
    Error,
}

#[derive(Clone)]
struct Entry {
    id: u64,
    kind: ToastKind,
    text: String,
}

/// The toast of the window. Home provides it; `surface` draws it.
#[derive(Clone, Copy)]
pub(crate) struct Toasts {
    owner: Owner,
    entry: Signal<Option<Entry>>,
    serial: Stored<u64>,
}

impl Toasts {
    pub(crate) fn mount() -> Self {
        Self {
            owner: Owner::current(),
            entry: signal_with(None, SignalEquals::Never),
            serial: stored(0),
        }
    }

    fn show(self, kind: ToastKind, text: impl Into<String>) {
        let text = text.into();
        let id = self.serial.get().wrapping_add(1);
        self.serial.replace(id);
        announce_for_accessibility(text.clone());
        self.entry.set(Some(Entry { id, kind, text }));
        // Failures stay longer: they ask the reader to do something.
        let stay = if kind == ToastKind::Error { 5 } else { 2 };
        self.owner.run(|| {
            set_timeout(Duration::from_secs(stay), move || {
                if self
                    .entry
                    .with_untracked(|entry| entry.as_ref().is_some_and(|entry| entry.id == id))
                {
                    self.entry.set(None);
                }
            });
        });
    }

    pub(crate) fn success(self, text: impl Into<String>) {
        self.show(ToastKind::Success, text);
    }

    pub(crate) fn error(self, text: impl Into<String>) {
        self.show(ToastKind::Error, text);
    }

    pub(crate) fn surface(self) -> impl IntoElement {
        let presence = Presence::bottom(move || self.entry.with(Option::is_some));
        // The notice keeps its last content while it slides out.
        let shown = signal_with(None::<Entry>, SignalEquals::Never);
        effect(move || {
            if let Some(entry) = self.entry.get() {
                shown.set(Some(entry));
            }
        });
        let kind = move || shown.with(|entry| entry.as_ref().map(|entry| entry.kind));
        when(
            move || presence.mounted(),
            move || {
                view()
                    .absolute()
                    .inset_0()
                    .items_center()
                    .justify_end()
                    .pb(space_xl())
                    .pointer_events(PointerEvents::None)
                    .child(
                        presence.surface(
                            super::glass(view())
                                .flex_row()
                                .items_center()
                                .gap(space_sm())
                                .max_w(relative(0.9))
                                .px(space_md())
                                .py(space_sm())
                                .rounded(radius_lg())
                                .border_1()
                                .border_color(border)
                                .shadow_lg()
                                .child(
                                    icon(Icon::Check)
                                        .size(icon_md())
                                        .flex_shrink_0()
                                        .text_color(success)
                                        .style_when(
                                            move || kind() != Some(ToastKind::Success),
                                            |style| style.hidden(),
                                        ),
                                )
                                .child(
                                    icon(Icon::InfoCircle)
                                        .size(icon_md())
                                        .flex_shrink_0()
                                        .text_color(danger)
                                        .style_when(
                                            move || kind() != Some(ToastKind::Error),
                                            |style| style.hidden(),
                                        ),
                                )
                                .child(
                                    text(move || {
                                        shown.with(|entry| {
                                            entry
                                                .as_ref()
                                                .map(|entry| entry.text.clone())
                                                .unwrap_or_default()
                                        })
                                    })
                                    .min_w_0()
                                    .text_role(TextRole::Body)
                                    .text_color(fg_primary),
                                ),
                        ),
                    )
            },
        )
    }
}
