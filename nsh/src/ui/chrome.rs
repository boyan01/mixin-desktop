//! The window chrome that the app draws itself.

use nsh_core::{element::desktop::title_bar as native_title_bar, prelude::*};

use super::tokens::{space_lg, title_bar_requested_height};

/// The row the top of every pane aligns to. The native height is authoritative;
/// full screen and windows without native controls report none.
fn title_bar_height() -> Points {
    let native = native_title_bar().top;
    if native.0 > 0.0 {
        native
    } else {
        title_bar_requested_height()
    }
}

/// The leading padding of a bar `offset` from the window's edge: the bars' inset,
/// or what clears the native window controls when they reach over the bar.
pub(crate) fn title_bar_leading(offset: Points) -> Points {
    px((native_title_bar().left.0 - offset.0).max(space_lg().0))
}

/// A strip of the title bar. A press on it drags the window and a double click
/// follows the system's title-bar action; the inputs and pressable children added
/// to it stay interactive.
pub(crate) fn title_bar() -> View {
    view()
        .min_h(title_bar_height)
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .window_drag(true)
}
