//! The chat's title bar: identity, app entry and panel toggles.
use mixin_desktop_api::ConversationListData;
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    i18n::{self, Key},
    inspector::Mode,
    navigation::ChatNavigation,
    ui,
};

pub(super) fn bar(
    row: Signal<ConversationListData>,
    group: bool,
    narrow: impl Fn() -> bool + Copy + 'static,
    back: impl Fn() + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let navigation = use_context::<ChatNavigation>().expect("Home provides chat navigation");
    let links =
        use_context::<crate::links::LinkNavigation>().expect("Home provides link navigation");
    // The row changes with every message; its identity rarely does, and these
    // parts rebuild views.
    let avatar = computed(move || {
        row.with(|row| {
            (
                row.name.clone(),
                row.avatar_url.clone(),
                row.category.clone(),
                row.group_avatars.clone(),
            )
        })
    });
    let badge =
        computed(move || row.with(|row| (row.is_verified, row.is_bot, row.membership.clone())));
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .px_6()
        .py_4()
        .bg(move || theme().background)
        .border_b_1()
        .border_color(move || theme().border)
        .child(
            ui::command()
                .style_when(move || !narrow(), |style| style.hidden())
                .on_press(back)
                .accessibility_label(move || Some(i18n.text(Key::Back).into()))
                .child(ui::icon(ui::Icon::ArrowLeft)),
        )
        .child(
            ui::command()
                .flex_1()
                .min_w_0()
                .px_0()
                .py_0()
                .gap_3()
                .justify_start()
                .accessibility_label(move || Some(i18n.text(Key::ChatInfo).into()))
                .on_press(move || navigation.toggle_panel(Mode::Info))
                .child(ui::conversation_avatar(
                    move || {
                        avatar.with(|_| ());
                        row.with_untracked(|row| Some(row.clone()))
                    },
                    px(40.0),
                ))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .gap_1()
                        .child(
                            view()
                                .flex_row()
                                .min_w_0()
                                .items_center()
                                .gap_1()
                                .child(
                                    text(move || row.with(|row| row.name.clone()))
                                        .min_w_0()
                                        .flex_shrink(1.0)
                                        .number_of_lines(1)
                                        .font_weight(FontWeight::SEMIBOLD),
                                )
                                .child(view().flex_shrink_0().child(dynamic(move || {
                                    let (verified, bot, membership) = badge.get();
                                    untracked(|| {
                                        crate::appearance::identity_badge(
                                            group,
                                            verified,
                                            bot,
                                            membership.as_deref(),
                                        )
                                    })
                                }))),
                        )
                        .child(
                            text(move || {
                                if group {
                                    i18n.members(row.with(|row| row.participant_count))
                                } else {
                                    row.with(|row| row.identity_number.clone())
                                }
                            })
                            .text_xs()
                            .text_color(move || theme().muted_foreground),
                        ),
                ),
        )
        .child(when(
            move || !group && row.with(|row| row.is_bot && !row.owner_id.is_empty()),
            move || {
                let open = move || i18n.tr("Open app", "打开应用");
                view()
                    .flex_none()
                    .tooltip(ui::hint(move || open().to_owned()))
                    .child(
                        ui::icon_button()
                            .size_10()
                            .px_1()
                            .accessibility_label(move || Some(open().into()))
                            .on_press(move || {
                                let (owner, conversation) = row.with_untracked(|row| {
                                    (row.owner_id.clone(), row.conversation_id.clone())
                                });
                                links.open(
                                    format!("mixin://apps/{owner}?action=open"),
                                    Some(conversation),
                                );
                            })
                            .child(ui::icon(ui::Icon::Bot).size_6()),
                    )
            },
        ))
        .child(panel_toggle(
            navigation,
            ui::Icon::Search,
            Key::SearchMessages,
            Mode::Search,
        ))
        .child(panel_toggle(
            navigation,
            ui::Icon::Info,
            Key::ChatInfo,
            Mode::Info,
        ))
}

fn panel_toggle(
    navigation: ChatNavigation,
    icon: ui::Icon,
    label: Key,
    mode: Mode,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::command()
        .size_10()
        .px_1()
        .accessibility_label(move || Some(i18n.text(label).into()))
        .selected(move || navigation.panel.get() == Some(mode))
        .selected_style(|style| style.bg(move || theme().accent))
        .on_press(move || navigation.toggle_panel(mode))
        .child(ui::icon(icon).size_6())
}
