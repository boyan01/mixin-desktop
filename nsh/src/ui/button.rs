//! Commands: `button` and its shorthands, and icon-only commands.

use nsh_base::{ButtonElement, theme};
use nsh_core::prelude::*;

use super::{
    state::{press_scale, pressable},
    tokens::{
        TextRole, Typography, accent, border, button_height, button_height_small, danger,
        fg_on_accent, fg_primary, opacity_solid_hover, opacity_solid_pressed, radius_full,
        radius_md, radius_sm, space_lg, space_md, space_sm, space_xs, status_fill,
    },
};

#[derive(Clone, Copy)]
pub(crate) enum ButtonTone {
    /// The one emphasized commitment of a decision area.
    Primary,
    /// An ordinary visible action, framed.
    Secondary,
    /// A quiet toolbar or inline action.
    Ghost,
    /// An action that destroys or removes.
    Danger,
}

#[derive(Clone, Copy)]
pub(crate) enum ButtonSize {
    Regular,
    Small,
}

/// A command. Framed tones use the control radius; a ghost has no frame at rest,
/// so its hover fill follows the rows it sits among.
pub(crate) fn button(tone: ButtonTone, size: ButtonSize) -> ButtonElement {
    let frame = pressable()
        .flex_row()
        .items_center()
        .justify_center()
        .text_role(TextRole::Body)
        .text_color(fg_primary);
    let frame = match size {
        ButtonSize::Regular => frame
            .min_h(button_height())
            .px(space_md())
            .py(space_sm())
            .gap(space_sm()),
        ButtonSize::Small => frame
            .min_h(button_height_small())
            .px(space_sm())
            .py(space_xs())
            .gap(space_xs()),
    };
    match tone {
        ButtonTone::Primary => frame
            .transition()
            .rounded(radius_sm())
            .px(space_lg())
            .text_emphasis()
            .bg(accent)
            .text_color(fg_on_accent)
            .hover(|s| s.bg(move || accent().opacity(opacity_solid_hover())))
            .active(|s| press_scale(s.bg(move || accent().opacity(opacity_solid_pressed())))),
        ButtonTone::Secondary => frame
            .rounded(radius_sm())
            .border_1()
            .border_color(border)
            .bg(move || theme().secondary)
            .hover(|s| s.bg(move || theme().muted))
            .active(|s| s.bg(move || theme().accent)),
        ButtonTone::Ghost => frame.rounded(radius_md()),
        ButtonTone::Danger => frame.rounded(radius_sm()).text_color(danger),
    }
}

/// The default commitment of a decision area.
pub(crate) fn primary() -> ButtonElement {
    button(ButtonTone::Primary, ButtonSize::Regular)
}

/// A labeled, framed action.
pub(crate) fn action(
    label: impl Fn() -> &'static str + 'static,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button(ButtonTone::Secondary, ButtonSize::Regular)
        .on_press(on_press)
        .child(text(label))
}

/// A light, accent-labeled command that invites rather than commits, such as
/// adding a contact. Flutter's `DialogAddOrJoinButton`.
pub(crate) fn pill(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    pressable()
        .flex_row()
        .items_center()
        .justify_center()
        .transition()
        .min_h(button_height_small())
        .px(space_lg())
        .rounded(radius_full())
        .bg(status_fill)
        .text_color(accent)
        .text_role(TextRole::Caption)
        .hover(|s| s.bg(move || status_fill().opacity(opacity_solid_hover())))
        .active(|s| press_scale(s.bg(move || status_fill().opacity(opacity_solid_pressed()))))
        .on_press(on_press)
        .child(text(label))
}

/// A quiet command whose content the caller supplies.
pub(crate) fn command() -> ButtonElement {
    button(ButtonTone::Ghost, ButtonSize::Regular)
}

/// A command read inside running text, such as a message's "Learn more".
pub(crate) fn link(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    command()
        .min_h_0()
        .px_0()
        .py_0()
        .justify_start()
        .text_color(accent)
        .on_press(on_press)
        .child(text(label))
}

/// An icon-only command inside content. Give it an accessible name.
pub(crate) fn icon_button() -> ButtonElement {
    command().transition().active(press_scale)
}

/// An icon-only command in a bar, round to match the search pill and the
/// composer. Size it at the call site and give it an accessible name.
pub(crate) fn round_button() -> ButtonElement {
    icon_button().min_h_0().px_0().py_0().rounded_full()
}
