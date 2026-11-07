//! Floating surfaces: popups, menus and tooltips. They and dialogs share one
//! treatment: `surface_popover`, the `border` and one shadow, at `radius_lg`, or
//! `radius_md` for a tooltip, which is too small for the larger corners.

use nsh_base::{MenuElement, MenuItemElement, PopoverElement, TooltipElement, menu_item, tooltip};
use nsh_core::{
    ContextMenuEvent,
    element::{
        StyleRefinement,
        desktop::{MenuItem, popup_menu_at},
    },
    prelude::*,
};

use super::{
    state::feedback_transition,
    tokens::{
        TextRole, Typography, border, fg_primary, opacity_disabled, radius_lg, radius_md, space_md,
        space_sm, space_xs, state_hover, state_pressed, surface_popover,
    },
};
use crate::motion;

pub(super) fn popup(style: StyleRefinement) -> StyleRefinement {
    motion::popup(style)
        .min_w_40()
        .p(space_xs())
        .rounded(radius_lg())
        .bg(surface_popover)
        .text_color(fg_primary)
        .border_1()
        .border_color(border)
        .shadow_lg()
}

/// A menu or context-menu command.
pub(crate) fn menu_row(label: impl Fn() -> &'static str + 'static) -> MenuItemElement {
    feedback_transition(menu_item())
        .px(space_md())
        .py(space_sm())
        .rounded(radius_md())
        .hover(|s| s.bg(state_hover))
        .focus(|s| s.bg(state_hover))
        .active(|s| s.bg(state_pressed))
        .disabled_style(|s| s.opacity(opacity_disabled()))
        .child(text(label).text_role(TextRole::Body))
}

/// The short hint shown beside a control whose label is hidden.
pub(crate) fn hint(label: impl Fn() -> String + 'static) -> TooltipElement {
    tooltip()
        .px(space_md())
        .py(space_sm())
        .rounded(radius_md())
        .bg(surface_popover)
        .text_color(fg_primary)
        .border_1()
        .border_color(border)
        .child(text(label).text_role(TextRole::Body))
}

pub(crate) fn menu() -> MenuElement {
    nsh_base::menu()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
}

/// Opens the OS menu of `items` on a secondary click over `content`. The items
/// are a snapshot made on each opening; none means no menu, which is how a
/// caller disables it.
pub(crate) fn native_context_menu(
    content: impl IntoElement + 'static,
    items: impl Fn() -> Vec<MenuItem> + 'static,
) -> View {
    view()
        .on_context_menu(move |event: &ContextMenuEvent| {
            let items = items();
            if !items.is_empty() {
                drop(popup_menu_at(items, event.page_position));
            }
        })
        .child(content)
}

/// Joins the non-empty `groups` with separators, so a hidden group leaves no
/// leading, trailing or doubled line.
pub(crate) fn menu_groups(groups: impl IntoIterator<Item = Vec<MenuItem>>) -> Vec<MenuItem> {
    let mut items = Vec::new();
    for group in groups.into_iter().filter(|group| !group.is_empty()) {
        if !items.is_empty() {
            items.push(MenuItem::separator());
        }
        items.extend(group);
    }
    items
}

/// A popover whose content reaches its edges: the content brings its own padding.
pub(crate) fn popover_flush() -> PopoverElement {
    popover().popup_style(|style| popup(style).p_0().overflow_hidden())
}

pub(crate) fn popover() -> PopoverElement {
    nsh_base::popover()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
}
