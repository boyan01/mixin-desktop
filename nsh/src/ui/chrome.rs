//! The window chrome that the app draws itself.

use nsh_core::{element::desktop::title_bar as native_title_bar, prelude::*};

use super::tokens::{
    bubble_in, glass_bar_height, glass_blur, glass_tint_opacity, radius_full, space_lg,
    title_bar_requested_height,
};

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

/// A surface that floats over the chat: Liquid Glass where the platform draws it
/// (macOS 26 and later), a blur under a translucent tint elsewhere. Use it only
/// for controls and navigation above content, never for content itself.
pub(crate) fn glass<T: Styled>(element: T) -> T {
    element
        .glass_effect(nsh_core::GlassEffect::Regular)
        .backdrop_blur(glass_blur())
        .bg(|| bubble_in().opacity(glass_tint_opacity()))
}

/// A glass capsule at least the floating chat chrome's height.
pub(crate) fn glass_pill() -> View {
    glass(view())
        .min_h(glass_bar_height())
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .rounded(radius_full())
}

/// A round glass control of the chrome's height. The glass stays on the
/// capsule, so the button's hover and selected fills stack on its tint.
pub(crate) fn glass_button(button: nsh_base::ButtonElement) -> View {
    glass_pill()
        .w(glass_bar_height())
        .justify_center()
        .child(button.size(glass_bar_height()))
}
