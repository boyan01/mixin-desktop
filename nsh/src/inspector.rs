//! Owns the right-hand destination; feature pages own their own state and queries.

mod pinned;
mod search;

use std::sync::Arc;

use mixin_desktop_api::AccountClient;
use nsh_core::prelude::*;

use crate::{conversation, ui};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Mode {
    Info,
    Search,
    Pinned,
}

pub(crate) fn screen(
    account: Arc<AccountClient>,
    id: String,
    mode: Mode,
    close: impl Fn() + Copy + 'static,
    target: Signal<Option<String>>,
    compact: impl Fn() -> bool + Copy + 'static,
) -> impl IntoElement {
    let panel = view()
        .w(ui::sidebar_width)
        .flex_shrink_0()
        .min_h_0()
        .style_with(move |s| if compact() { s.flex_1().min_w_0() } else { s })
        .bg(ui::surface_popover)
        .border_l_1()
        .border_color(ui::border);
    if mode == Mode::Info {
        return panel
            .child(conversation::screen(account, id, target, close))
            .into_element();
    }
    if mode == Mode::Search {
        return panel
            .child(search::screen(account, id, close, target))
            .into_element();
    }
    panel
        .child(pinned::screen(account, id, close, target))
        .into_element()
}
