//! Production Markdown readers over offline fixtures, without account services.
use nsh_core::prelude::*;

use super::{Actions, fixtures};
use crate::{appearance::Appearance, ui};

pub(super) const ACTIONS: &[&str] = &["Larger text"];

pub(super) fn reader(actions: Actions) -> View {
    scene(actions, fixtures::POST_PROSE)
}

pub(super) fn blocks(actions: Actions) -> View {
    scene(actions, fixtures::POST_BLOCKS)
}

fn scene(actions: Actions, content: &'static str) -> View {
    let appearance = Appearance::detached();
    actions.on(
        "Larger text",
        move || appearance.preview_font(0.0),
        move || appearance.preview_font(4.0),
    );
    view()
        .flex_1()
        .min_h_0()
        .bg(ui::surface_pane)
        .child(ui::markdown_reader(
            move || content.into(),
            move || appearance.primary_size(),
            |_| {},
        ))
}
