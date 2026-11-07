//! Selectable rows of the navigation rail and of conversation-sized lists.

use nsh_base::ButtonElement;
use nsh_core::{AccessibilityState, prelude::*};

use super::{
    state::{feedback_transition, pressable},
    tokens::{
        TextRole, Typography, fg_primary, list_row_height, nav_row_height, radius_md, space_2xs,
        space_md, space_sm, state_hover, state_selected,
    },
};

/// What every selectable row shares. Rows align to the start and lay out their own
/// content; the fill comes from the state style.
pub(super) fn row_frame() -> ButtonElement {
    pressable()
        .flex_row()
        .items_center()
        .justify_start()
        .rounded(radius_md())
        .text_role(TextRole::Body)
        .text_color(fg_primary)
}

/// A row of the navigation rail, also the frame of a compact pressable identity.
pub(crate) fn nav_row() -> ButtonElement {
    row_frame()
        .min_h(nav_row_height())
        .px(space_sm())
        .gap(space_sm())
}

/// A selectable row of a conversation-sized list. With its margins it is one list
/// cell of `list_row_height`.
pub(crate) fn list_row() -> ButtonElement {
    let height = list_row_height().0 - 2.0 * space_2xs().0;
    row_frame()
        .min_h(px(height))
        .mx(space_sm())
        .my(space_2xs())
        .px(space_md())
        .gap(space_md())
}

/// An option of a listbox driven from an input. It never takes focus, as NSH's
/// combobox options do, so editing stays in the input; a button would move it.
pub(crate) fn option_row(selected: impl Fn() -> bool + Copy + 'static) -> View {
    feedback_transition(view())
        .role(AccessibilityRole::Option)
        .focusable(false)
        .accessibility_state(move || AccessibilityState {
            selected: selected(),
            ..Default::default()
        })
        .flex_row()
        .items_center()
        .justify_start()
        .px(space_md())
        .py(space_sm())
        .gap(space_sm())
        .rounded(radius_md())
        .style_when(selected, |s| s.bg(state_selected))
        // Hover outranks the row's own style, so it keeps the selection.
        .hover(move |s| {
            s.bg(move || {
                if selected() {
                    state_selected()
                } else {
                    state_hover()
                }
            })
        })
}
