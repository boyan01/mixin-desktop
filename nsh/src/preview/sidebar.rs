//! The navigation rail with fixed profile and unread counts. It lists no circles
//! and does not persist Collapse.

use nsh_core::prelude::*;

use super::{Actions, fixtures};
use crate::{navigation::Navigation, sidebar::Rail, ui};

pub(super) const ACTIONS: &[&str] = &["Collapse", "Expand", "Error", "Reset"];

pub(super) fn scene(actions: Actions) -> View {
    let failed = signal(false);
    actions.on("Error", move || failed.set(false), move || failed.set(true));
    actions.on("Reset", move || failed.set(true), move || failed.set(false));
    let nav = Navigation::new();
    let rail = Rail::detached(
        nav,
        signal_with(fixtures::profile(), SignalEquals::Never),
        signal(fixtures::unseen_counts()),
    );
    for (name, collapsed) in [("Collapse", true), ("Expand", false)] {
        actions.on(
            name,
            move || nav.collapsed.set(!collapsed),
            move || {
                nav.animate_width();
                nav.collapsed.set(collapsed);
            },
        );
    }
    view()
        .flex_1()
        .flex_row()
        .bg(ui::surface_pane)
        .child(view().w(move || nav.width()).flex_shrink_0().child(
            rail.content_with_feedback(when(failed, || crate::sidebar::preference_error(|| {}))),
        ))
        .child(view().flex_1())
}
