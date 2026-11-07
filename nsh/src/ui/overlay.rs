//! Floating surfaces: popups, menus, tooltips and dialogs. They share one
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
        TextRole, Typography, border, fg_primary, opacity_disabled, radius_lg, radius_md,
        scrim_fill, space_lg, space_md, space_sm, space_xl, space_xs, state_hover, state_pressed,
        surface_popover,
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

pub(crate) fn popover() -> PopoverElement {
    nsh_base::popover()
        .popup_style(popup)
        .popup_starting_style(motion::popup_start)
        .popup_ending_style(motion::popup_end)
}

/// The owner retains the surface for exit; callers guard keyboard submissions.
pub(crate) fn modal(
    presence: motion::Presence,
    close: impl Fn() + 'static,
    card: View,
) -> nsh_core::element::Modal {
    // Release background editing after native presentation records its restore target.
    dismiss_keyboard();
    let close = stored(close);
    nsh_base::dialog()
        .animation(nsh_core::ModalAnimation::None)
        .close_on_escape(false)
        .close_on_backdrop_press(false)
        .on_cancel(move || {
            if presence.interactive() {
                close.with(|close| close());
            }
        })
        .flex_1()
        .child(
            view()
                .flex_1()
                .p(space_xl())
                .items_center()
                .justify_center()
                .child(presence.backdrop(scrim().on_press(move || {
                    if presence.interactive() {
                        close.with(|close| close());
                    }
                })))
                .child(presence.surface(card)),
        )
}

/// The dimming layer behind modal surfaces.
pub(crate) fn scrim() -> View {
    view().absolute().inset_0().bg(scrim_fill)
}

/// The surface of a small dialog; size it further at the call site.
pub(crate) fn modal_card() -> View {
    view()
        .w_80()
        .max_w(relative(1.0))
        .max_h(relative(0.9))
        .min_h_0()
        .p(space_xl())
        .gap(space_lg())
        .rounded(radius_lg())
        .bg(surface_popover)
        .text_color(fg_primary)
        .border_1()
        .border_color(border)
        .shadow_lg()
}

pub(crate) fn modal_title(title: impl IntoReactive<SharedString>) -> Text {
    text(title).flex_none().text_role(TextRole::Headline)
}
