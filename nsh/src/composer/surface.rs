//! Presentation shared by the live composer and account-free previews.
use crate::ui;
use nsh_base::ButtonElement;
use nsh_core::prelude::*;

pub(crate) fn frame() -> View {
    view()
        .rounded(ui::radius_composer())
        .bg(ui::bubble_in)
        .border_1()
        .border_color(ui::input_border)
}

pub(crate) fn editor(
    files: impl IntoElement,
    input: impl IntoElement,
    tools: impl IntoElement,
    submit: impl IntoElement,
) -> View {
    frame()
        .child(files)
        .child(view().px_1().pt_1().child(input))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .px_2()
                .pb_2()
                .gap_2()
                .child(tools)
                .child(submit),
        )
}

pub(crate) fn send_button() -> ButtonElement {
    ui::primary()
        .size_8()
        .flex_shrink_0()
        .min_h_0()
        .p_0()
        .rounded_full()
}

#[cfg(debug_assertions)]
pub(crate) fn preview_tool() -> ButtonElement {
    super::tool_button()
}

pub(crate) fn hinted(content: impl IntoElement, label: impl Fn() -> String + 'static) -> View {
    view()
        .flex_shrink_0()
        .tooltip(ui::hint(label))
        .child(content)
}

/// A single native menu target, without an inner button competing for presses.
pub(crate) fn menu_trigger() -> View {
    view()
        .size(ui::button_height_small())
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .justify_center()
        .rounded(ui::radius_md())
        .role(AccessibilityRole::Button)
        .focusable(true)
        .text_color(ui::fg_secondary)
        .hover(|s| s.bg(ui::state_hover))
        .active(|s| s.bg(ui::state_pressed))
        .focus(ui::ring)
}
