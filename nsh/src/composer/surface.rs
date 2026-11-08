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
    frame().child(files).child(
        view()
            .flex_row()
            .items_end()
            .p_2()
            .gap_2()
            .child(tools)
            .child(view().flex_1().min_w_0().child(input))
            .child(submit),
    )
}

/// Both actions retain their identity and share one fixed keyboard/pointer slot.
pub(crate) fn primary_action(
    sending: Memo<bool>,
    send: impl IntoElement,
    record: impl IntoElement,
) -> View {
    let send_opacity = animated_value(if sending.get_untracked() { 1.0 } else { 0.0 });
    let record_opacity = animated_value(if sending.get_untracked() { 0.0 } else { 1.0 });
    effect(move || {
        let sending = sending.get();
        let ms = crate::motion::duration(crate::motion::FEEDBACK_MS);
        let _ = send_opacity.timing(
            if sending { 1.0 } else { 0.0 },
            ms,
            nsh_core::animation::Easing::EaseOut,
        );
        let _ = record_opacity.timing(
            if sending { 0.0 } else { 1.0 },
            ms,
            nsh_core::animation::Easing::EaseOut,
        );
    });
    let layer = move |active: Memo<bool>| {
        view()
            .absolute()
            .inset_0()
            .disabled(move || !active.get())
            .accessibility_elements_hidden(move || !active.get())
            .pointer_events(move || {
                if active.get() {
                    nsh_core::PointerEvents::Auto
                } else {
                    nsh_core::PointerEvents::None
                }
            })
    };
    view()
        .relative()
        .size_8()
        .flex_shrink_0()
        .child(layer(sending).bind_opacity(&send_opacity).child(send))
        .child(
            layer(computed(move || !sending.get()))
                .bind_opacity(&record_opacity)
                .child(record),
        )
}

/// Keep the editor mounted so switching modes never loses its document or caret.
pub(crate) fn text_mode(recording: Memo<bool>, editor: View) -> View {
    let opacity = animated_value(if recording.get_untracked() { 0.0 } else { 1.0 });
    effect(move || {
        let _ = opacity.timing(
            if recording.get() { 0.0 } else { 1.0 },
            crate::motion::duration(crate::motion::FEEDBACK_MS),
            nsh_core::animation::Easing::EaseOut,
        );
    });
    editor
        .bind_opacity(&opacity)
        .disabled(recording)
        .accessibility_elements_hidden(recording)
        .pointer_events(move || {
            if recording.get() {
                nsh_core::PointerEvents::None
            } else {
                nsh_core::PointerEvents::Auto
            }
        })
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
