//! Grouped surfaces of rows, as a settings page has them.

use nsh_base::{ButtonElement, CheckboxElement};
use nsh_core::prelude::*;

use super::{
    form::{NO_LABEL, checkbox, switch},
    icon::{Icon, icon},
    rows::row_frame,
    state::inset_ring,
    tokens::{
        TextRole, Typography, fg_secondary, icon_lg, icon_sm, radius_md, row_height, space_lg,
        space_md, space_sm, surface_card,
    },
};

/// A group of rows, one tone above its surface. Rows reach its edges and it clips
/// them, so a selected fill follows its corners.
pub(crate) fn card() -> View {
    view()
        .rounded(radius_md())
        .overflow_hidden()
        .bg(surface_card)
}

/// A pressable row of a `card`. It draws its focus inside, because the card clips.
pub(crate) fn card_row() -> ButtonElement {
    row_frame()
        .rounded_none()
        .min_h(row_height())
        .px(space_lg())
        .gap(space_sm())
        .focus(inset_ring)
}

/// A labeled row of a `card`. Add `chevron` when it leads somewhere.
pub(crate) fn settings_row(
    leading: Option<Icon>,
    label: impl Fn() -> &'static str + 'static,
) -> ButtonElement {
    let row = card_row();
    match leading {
        Some(leading) => row.child(icon(leading).size(icon_lg())),
        None => row,
    }
    .child(
        text(label)
            .text_role(TextRole::Label)
            .flex_1()
            .min_w_0()
            .number_of_lines(1),
    )
}

pub(crate) fn chevron() -> Svg {
    icon(Icon::ChevronRight)
        .size(icon_sm())
        .text_color(fg_secondary)
}

/// A static row of a `card`: a label, with its value or control added after it.
pub(crate) fn static_row(label: impl Fn() -> &'static str + 'static) -> View {
    view()
        .min_h(row_height())
        .px(space_lg())
        .py(space_sm())
        .flex_row()
        .items_center()
        .gap(space_md())
        .child(text(label).text_role(TextRole::Label).flex_1().min_w_0())
}

/// A row of a `card` that is one independent choice, with its content added after
/// the mark. It draws its focus inside, because the card clips.
pub(crate) fn checkbox_row(
    checked: impl Fn() -> bool + Copy + 'static,
    disabled: impl Fn() -> bool + 'static,
    on_change: impl Fn(bool) + 'static,
) -> CheckboxElement {
    checkbox(NO_LABEL, checked, disabled, on_change)
        .rounded_none()
        .min_h(row_height())
        .px(space_lg())
        .focus(inset_ring)
}

/// A static row holding a switch.
pub(crate) fn toggle_row(
    label: impl Fn() -> &'static str + Copy + 'static,
    checked: impl Fn() -> bool + 'static,
    disabled: impl Fn() -> bool + 'static,
    change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    static_row(label).child(switch(label, checked, disabled, change))
}
