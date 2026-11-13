//! Account-free mention picker over a chat backdrop.
use super::Actions;
use crate::ui;
use nsh_core::prelude::*;

pub(super) const ACTIONS: &[&str] = &[];

pub(super) fn scene(_: Actions) -> View {
    view()
        .flex_1()
        .bg(ui::surface_chat)
        .justify_end()
        .p_4()
        .gap_2()
        .child(text("Let’s review the design together.").text_color(ui::fg_primary))
        .child(crate::composer::mentions::preview())
        .child(text("@al").text_color(ui::fg_primary).p_3())
}
