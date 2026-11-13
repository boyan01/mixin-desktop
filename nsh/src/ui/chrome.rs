//! The window chrome that the app draws itself.

use nsh_core::{element::desktop::title_bar as native_title_bar, prelude::*};

use super::tokens::{
    bubble_in, edge_fade_height, glass_bar_height, glass_blur, glass_edge, glass_tint_opacity,
    space_lg, title_bar_requested_height,
};

/// The row the top of every pane aligns to. The native height is authoritative;
/// full screen and windows without native controls report none.
pub(crate) fn title_bar_height() -> Points {
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
    let element = element
        .glass_effect(nsh_core::GlassEffect::Regular)
        .backdrop_blur(glass_blur())
        .border_1()
        .border_color(glass_edge);
    // A fill draws above the material and hides it. macOS draws Liquid Glass
    // (26 and later) or a system material under the blur, both legible alone.
    if cfg!(target_os = "macos") {
        element
    } else {
        element.bg(|| bubble_in().opacity(glass_tint_opacity()))
    }
}

/// The corner of a glass capsule: half its height. Glass takes the radius as
/// its shape, and a "full" radius far past the height draws it wrongly.
pub(crate) fn glass_radius() -> Points {
    px(glass_bar_height().0 / 2.0)
}

/// A glass capsule of the floating chat chrome's height; every capsule in a
/// row of chrome shares it, the border included.
pub(crate) fn glass_pill() -> View {
    glass(view())
        .h(glass_bar_height())
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .rounded(glass_radius())
}

/// A round glass control of the chrome's height. The button is the glass, so
/// its hover and selected fills draw on the material.
pub(crate) fn glass_button(button: nsh_base::ButtonElement) -> nsh_base::ButtonElement {
    glass(button)
        .size(glass_bar_height())
        .flex_shrink_0()
        .rounded(glass_radius())
}

/// The scroll edge effect of a scrolling region: a thin band at its very edge
/// where the page's color (`color`) fades to fully transparent, so content
/// leaves the region softly instead of being cut. Only the edge: the scroll
/// padding under floating chrome stays clear. Place it in a parent pinned to
/// that edge, behind the chrome.
pub(crate) fn edge_fade(top: bool, color: impl Fn() -> AdaptiveColor + Copy + 'static) -> View {
    let fade = view()
        .absolute()
        .left_0()
        .right_0()
        .h(edge_fade_height())
        .pointer_events(nsh_core::PointerEvents::None);
    // The shape first: the stops apply to the gradient it declares.
    let fade = if top {
        fade.top_0().bg_linear_to_b()
    } else {
        fade.bottom_0().bg_linear_to_t()
    };
    // Eased: most of the color goes in the first third, then it tails off.
    fade.gradient_from(color)
        .gradient_via(move || color().opacity(0.3))
        .gradient_via_position(percentage(0.35))
        .gradient_to(move || color().opacity(0.0))
}
