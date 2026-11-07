//! Dialogs: the blocking frame and its card, and the layout of a person in one.
//! The profile parts keep the measures of Flutter's `UserDialog`.

use nsh_base::ButtonElement;
use nsh_core::{element::Modal, prelude::*};

use super::{
    avatar::{AvatarSize, avatar_view},
    button::round_button,
    icon::{Icon, icon},
    tokens::{
        TextRole, Typography, border, danger, dialog_close_size, dialog_width_compact,
        dialog_width_regular, dialog_width_wide, fg_primary, fg_secondary, icon_lg,
        profile_action_icon, profile_action_inset, profile_action_size, profile_bio_height,
        radius_lg, radius_md, scrim_fill, space_lg, space_md, space_sm, space_xl, space_xs,
        surface_card, surface_popover,
    },
};
use crate::{i18n::tr, motion};

#[derive(Clone, Copy)]
pub(crate) enum DialogWidth {
    /// A decision, a short form or a person.
    Compact,
    /// A list or a longer form.
    Regular,
    /// Room for a document.
    Wide,
}

impl DialogWidth {
    fn points(self) -> Points {
        match self {
            Self::Compact => dialog_width_compact(),
            Self::Regular => dialog_width_regular(),
            Self::Wide => dialog_width_wide(),
        }
    }
}

/// A dialog that a press on the scrim dismisses. The owner retains the surface
/// for exit; callers guard keyboard submissions.
pub(crate) fn dialog(presence: motion::Presence, close: impl Fn() + 'static, card: View) -> Modal {
    let close = stored(close);
    let dismiss = move || {
        if presence.interactive() {
            close.with(|close| close());
        }
    };
    frame(presence, dismiss, scrim().on_press(dismiss), card)
}

/// A `dialog` that a stray press on the scrim must not discard, such as a
/// selection in progress.
pub(crate) fn held_dialog(
    presence: motion::Presence,
    close: impl Fn() + 'static,
    card: View,
) -> Modal {
    let dismiss = move || {
        if presence.interactive() {
            close();
        }
    };
    frame(presence, dismiss, scrim(), card)
}

fn frame(
    presence: motion::Presence,
    dismiss: impl Fn() + 'static,
    scrim: View,
    card: View,
) -> Modal {
    // Release background editing after native presentation records its restore target.
    dismiss_keyboard();
    nsh_base::dialog()
        .animation(nsh_core::ModalAnimation::None)
        .close_on_escape(false)
        .close_on_backdrop_press(false)
        .on_cancel(dismiss)
        .flex_1()
        .child(
            view()
                .flex_1()
                .p(space_xl())
                .items_center()
                .justify_center()
                .child(presence.backdrop(scrim))
                .child(presence.surface(card)),
        )
}

/// The dimming layer behind modal surfaces.
pub(crate) fn scrim() -> View {
    view().absolute().inset_0().bg(scrim_fill)
}

/// The look of a `dialog_card` as a style, for a card that changes shape with what
/// it shows.
pub(crate) fn dialog_surface<S: Styled>(style: S, width: DialogWidth) -> S {
    style
        .w(width.points())
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

pub(crate) fn dialog_card(width: DialogWidth) -> View {
    dialog_surface(view(), width)
}

pub(crate) fn dialog_title(title: impl IntoReactive<SharedString>) -> Text {
    text(title).flex_none().text_role(TextRole::Headline)
}

/// The close command in a card's top corner, for a dialog with no Cancel.
pub(crate) fn dialog_close(on_press: impl Fn() + 'static) -> ButtonElement {
    round_button()
        .flex_none()
        .self_end()
        .mt(space_md())
        .mr(space_md())
        .size(dialog_close_size())
        .accessibility_label(move || Some(tr("Close", "关闭").into()))
        .on_press(on_press)
        .child(icon(Icon::Close).size(icon_lg()))
}

/// A person, centered: picture, name and a muted line, which is left out when
/// empty. Add a `pill`, `profile_bio`, `profile_notice` and `profile_actions`
/// after it, in that order. `id` picks the placeholder's color.
pub(crate) fn profile_card(
    size: AvatarSize,
    id: impl Fn() -> String + 'static,
    name: impl Fn() -> String + Copy + 'static,
    avatar: impl Fn() -> String + 'static,
    subtitle: impl Fn() -> String + Copy + 'static,
    badge: impl IntoElement + 'static,
) -> View {
    view()
        .w_full()
        .items_center()
        .child(avatar_view(id, move || name().into(), avatar, size))
        .child(
            view()
                .max_w(relative(1.0))
                .mt(space_sm())
                .px_8()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(space_xs())
                .child(
                    text(name)
                        .flex_shrink(1.0)
                        .min_w_0()
                        .text_role(TextRole::Label)
                        .text_center()
                        .select_text(),
                )
                .child(badge),
        )
        .child(
            text(subtitle)
                .mt(space_xs())
                .text_role(TextRole::Caption)
                .text_color(fg_secondary)
                .select_text()
                .style_when(move || subtitle().is_empty(), |style| style.hidden()),
        )
}

/// A person's own words. A long one scrolls in place.
pub(crate) fn profile_bio(content: impl IntoReactive<SharedString>) -> impl IntoElement {
    scroll().w_full().mt_5().max_h(profile_bio_height()).child(
        view()
            .px_9()
            .items_center()
            .child(text(content).text_role(TextRole::Body).select_text()),
    )
}

/// What stands in for the details of a person who is gone.
pub(crate) fn profile_notice(content: impl IntoReactive<SharedString>) -> View {
    view().w_full().mt_5().px_5().child(
        view()
            .items_center()
            .px_5()
            .py(space_md())
            .rounded(radius_md())
            .bg(surface_card)
            .child(
                text(content)
                    .text_role(TextRole::Body)
                    .text_center()
                    .text_color(danger),
            ),
    )
}

/// The row of `profile_action`s closing a `profile_card`. Center a lone one.
pub(crate) fn profile_actions() -> View {
    view()
        .w_full()
        .mt(space_xl())
        .px(profile_action_inset())
        .flex_row()
        .items_center()
        .justify_between()
}

pub(crate) fn profile_action(
    glyph: Icon,
    label: impl Fn() -> &'static str + 'static,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    round_button()
        .flex_none()
        .size(profile_action_size())
        .accessibility_label(move || Some(label().into()))
        .on_press(on_press)
        .child(icon(glyph).size(profile_action_icon()))
}
