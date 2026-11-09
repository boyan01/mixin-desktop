//! Shared contact/chat selection surfaces. Callers own selection and commands.
use nsh_base::{ButtonElement, Input};
use nsh_core::prelude::*;

use super::{self as ui, Typography as _};
use crate::i18n::tr;

pub(crate) fn dialog() -> View {
    ui::dialog_card(ui::DialogWidth::Regular)
        .w(ui::picker_width())
        .h(ui::picker_height())
        .p_0()
        .gap_0()
        .overflow_hidden()
}

/// Equal side slots keep the title centered when the commitment is absent.
pub(crate) fn header(
    title: impl Fn() -> &'static str + 'static,
    count: impl Fn() -> String + 'static,
    leading: ButtonElement,
    action: impl IntoElement,
) -> View {
    view()
        .flex_none()
        .flex_row()
        .items_center()
        .px(ui::space_xl())
        .pt(ui::space_lg())
        .pb(ui::space_md())
        .child(view().w_20().items_start().child(leading))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap(ui::space_2xs())
                .child(ui::dialog_title(title).number_of_lines(1))
                .child(
                    text(count)
                        .text_role(ui::TextRole::Caption)
                        .text_color(ui::fg_secondary),
                ),
        )
        .child(view().w_20().items_end().child(action))
}

pub(crate) fn close(close: impl Fn() + 'static) -> ButtonElement {
    ui::round_button()
        .size_8()
        .accessibility_label(|| Some(tr("Close", "关闭").into()))
        .child(ui::icon(ui::Icon::Close).size(ui::icon_lg()))
        .on_press(close)
}

pub(crate) fn commitment() -> ButtonElement {
    ui::command()
        .px_0()
        .text_color(ui::accent)
        .text_role(ui::TextRole::Title)
}

pub(crate) fn search(input: Input) -> View {
    view()
        .flex_none()
        .px(ui::space_xl())
        .pb(ui::space_md())
        .child(
            view()
                .flex_row()
                .flex_none()
                .h(ui::field_height())
                .min_h(ui::field_height())
                .items_center()
                .gap(ui::space_sm())
                .px(ui::space_lg())
                .rounded_full()
                .bg(ui::picker_search_fill)
                .focus_within(ui::ring)
                .child(
                    ui::icon(ui::Icon::Search)
                        .size(ui::icon_md())
                        .text_color(ui::fg_secondary),
                )
                .child(input.h(ui::field_height()).min_h(ui::field_height())),
        )
}

pub(crate) fn section(title: impl Fn() -> &'static str + 'static) -> Text {
    text(title)
        .text_role(ui::TextRole::Title)
        .text_color(ui::fg_primary)
        .px(ui::space_xl())
        .pt(ui::space_lg())
        .pb(ui::space_sm())
}

pub(crate) fn row_height() -> Points {
    ui::row_height() + ui::space_sm()
}

pub(crate) fn row(
    id: impl Fn() -> String + 'static,
    name: impl Fn() -> String + Copy + 'static,
    avatar: impl Fn() -> String + 'static,
    selected: impl Fn() -> bool + Copy + 'static,
    multiple: bool,
) -> ButtonElement {
    ui::command()
        .min_h(row_height())
        .mx(ui::space_md())
        .px(ui::space_md())
        .gap(ui::space_lg())
        .justify_start()
        .selected(selected)
        .selected_style(|style| style.bg(ui::surface_popover))
        .child(when(
            move || multiple,
            move || {
                view()
                    .size_4()
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .border_color(ui::fg_secondary)
                    .items_center()
                    .justify_center()
                    .style_when(selected, |style| {
                        style.bg(ui::accent).border_color(ui::accent)
                    })
                    .child(
                        ui::icon(ui::Icon::Check)
                            .size(ui::icon_xs())
                            .text_color(ui::fg_on_accent)
                            .style_when(move || !selected(), |style| style.invisible()),
                    )
            },
        ))
        .child(ui::avatar_view(
            id,
            move || name().into(),
            avatar,
            ui::AvatarSize::Large,
        ))
        .child(
            text(name)
                .text_role(ui::TextRole::Label)
                .number_of_lines(1)
                .min_w_0()
                .flex_1(),
        )
}

pub(crate) fn selected_person(
    id: impl Fn() -> String + 'static,
    name: impl Fn() -> String + Copy + 'static,
    avatar: impl Fn() -> String + 'static,
    disabled: impl Fn() -> bool + 'static,
    remove: impl Fn() + 'static,
) -> View {
    view()
        .w_20()
        .items_center()
        .gap(ui::space_xs())
        .child(
            view()
                .size_16()
                .relative()
                .items_center()
                .justify_center()
                .child(ui::avatar_view(
                    id,
                    move || name().into(),
                    avatar,
                    ui::AvatarSize::Large,
                ))
                .child(
                    ui::round_button()
                        .absolute()
                        .right_0()
                        .top_0()
                        .size_6()
                        .disabled(disabled)
                        .bg(ui::fg_secondary)
                        .text_color(ui::fg_on_accent)
                        .accessibility_label(move || {
                            Some(format!("{} {}", tr("Remove", "移除"), name()).into())
                        })
                        .on_press(remove)
                        .child(ui::icon(ui::Icon::Close).size(ui::icon_xs())),
                ),
        )
        .child(
            text(name)
                .text_role(ui::TextRole::Body)
                .number_of_lines(1)
                .text_center()
                .w_full(),
        )
}
