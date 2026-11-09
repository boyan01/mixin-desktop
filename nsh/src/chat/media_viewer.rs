//! Shared photo/video viewer chrome. Playback and image geometry stay with their owners.
use mixin_desktop_api::MessageListView;
use nsh_core::prelude::*;

use crate::{
    i18n,
    ui::{self, Typography},
};

pub(super) fn button(
    icon: ui::Icon,
    label: impl Fn() -> &'static str + Copy + 'static,
    on_press: impl Fn() + 'static,
    disabled: impl Fn() -> bool + 'static,
) -> View {
    view()
        .flex_none()
        .size_9()
        .items_center()
        .justify_center()
        .pointer_events(PointerEvents::Auto)
        .tooltip(ui::hint(move || label().to_owned()))
        .child(
            ui::round_button()
                .size_9()
                .text_color(ui::media_overlay_foreground)
                .hover(|style| style.bg(|| ui::media_overlay_foreground().opacity(0.18)))
                .disabled(disabled)
                .accessibility_label(move || Some(label().into()))
                .on_press(on_press)
                .child(ui::icon(icon).size(ui::icon_lg())),
        )
}

pub(super) fn header(
    selected: impl Fn() -> Option<MessageListView> + Copy + 'static,
    tools: impl Fn() -> View + Copy + 'static,
    close: impl Fn() + Copy + 'static,
    compact: impl Fn() -> bool + Copy + 'static,
) -> View {
    let i18n = i18n::current();
    let identity = move || {
        view()
            .flex_1()
            .min_w_0()
            .flex_row()
            .items_center()
            .gap(ui::space_lg())
            .child(ui::avatar_view(
                move || selected().map(|row| row.sender_id).unwrap_or_default(),
                move || {
                    selected()
                        .map(|row| row.sender_name)
                        .unwrap_or_default()
                        .into()
                },
                move || {
                    selected()
                        .map(|row| row.sender_avatar_url)
                        .unwrap_or_default()
                },
                ui::AvatarSize::Bar,
            ))
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .child(
                        text(move || selected().map(|row| row.sender_name).unwrap_or_default())
                            .text_role(ui::TextRole::Title)
                            .number_of_lines(1)
                            .text_color(ui::media_overlay_foreground),
                    )
                    .child(
                        text(move || {
                            selected()
                                .and_then(|row| row.sender_identity_number)
                                .unwrap_or_default()
                        })
                        .text_role(ui::TextRole::Body)
                        .number_of_lines(1)
                        .text_color(ui::viewer_secondary),
                    ),
            )
    };
    let close = move || {
        button(
            ui::Icon::CloseCircle,
            move || i18n.tr("Close", "关闭"),
            close,
            || false,
        )
    };
    let row = || {
        view()
            .min_h(ui::viewer_toolbar_height())
            .flex_row()
            .items_center()
            .gap(ui::space_lg())
            .px(ui::space_xl())
            .pl(move || ui::title_bar_leading(px(0.0)))
            .py(ui::space_sm())
    };
    view()
        .flex_none()
        .bg(ui::viewer_toolbar_fill)
        .window_drag(true)
        .child(
            when(compact, move || {
                view()
                    .child(row().child(identity()).child(close()))
                    .child(tools().self_end().px(ui::space_xl()).pb(ui::space_sm()))
            })
            .otherwise(move || row().child(identity()).child(tools()).child(close())),
        )
}
