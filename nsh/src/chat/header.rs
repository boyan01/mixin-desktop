//! The chat's title bar: glass capsules for identity, the latest pin when the
//! chat is wide enough, the app entry and the panel toggles, floating over the
//! timeline.
use mixin_desktop_api::ConversationListData;
use nsh_core::prelude::*;

use crate::{
    i18n::{self, Key},
    navigation::ChatNavigation,
    navigation::Panel,
    ui::{self, Typography},
};

pub(super) fn bar(
    row: Signal<ConversationListData>,
    group: bool,
    narrow: impl Fn() -> bool + Copy + 'static,
    back: impl Fn() + 'static,
    pinned: super::pinned::Pinned,
    inline_pin: Memo<bool>,
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
    let rail = use_context::<crate::navigation::Navigation>().expect("Home provides navigation");
    ui::title_bar()
        .gap_2()
        // Alone in the window, the bar reaches the native controls over a narrow rail.
        .pl(move || {
            if narrow() {
                ui::title_bar_leading(rail.width())
            } else {
                ui::space_lg()
            }
        })
        .pr_3()
        .py_2()
        .child(
            ui::glass_button(
                ui::round_button()
                    .on_press(back)
                    .accessibility_label(move || Some(i18n.text(Key::Back).into()))
                    .child(ui::icon(ui::Icon::ArrowLeft)),
            )
            .style_when(move || !narrow(), |style| style.hidden()),
        )
        .child(
            ui::glass_pill().min_w_0().flex_shrink(1.0).child(
                ui::nav_row()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .flex_shrink(1.0)
                    .rounded(ui::radius_full())
                    .pl_1()
                    .pr_4()
                    .py_0()
                    .gap_2()
                    .accessibility_label(move || Some(i18n.text(Key::ChatInfo).into()))
                    .on_press(move || navigation.toggle_panel(Panel::Info))
                    .child(ui::conversation_avatar(
                        move || {
                            avatar.with(|_| ());
                            row.with_untracked(|row| Some(row.clone()))
                        },
                        ui::AvatarSize::Compact,
                    ))
                    .child(
                        view()
                            .flex_shrink(1.0)
                            .min_w_0()
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
                                            .text_role(ui::TextRole::Title),
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
                                    })))
                                    .child(when(
                                        move || row.with(|row| row.expire_in > 0),
                                        || {
                                            ui::icon(ui::Icon::Disappearing)
                                                .text_color(ui::fg_secondary)
                                        },
                                    )),
                            )
                            .child(
                                text(move || {
                                    if group {
                                        i18n.members(row.with(|row| row.participant_count))
                                    } else {
                                        row.with(|row| row.identity_number.clone())
                                    }
                                })
                                .text_role(ui::TextRole::Caption)
                                .number_of_lines(1)
                                .text_color(ui::fg_secondary),
                            ),
                    ),
            ),
        )
        // The identity hugs its content, leaving the bar's middle to drag the window.
        .child(
            view()
                .flex_1()
                .min_w_0()
                .flex_row()
                .child(when(move || inline_pin.get(), move || pinned.view(1))),
        )
        .child(when(
            move || !group && row.with(|row| row.is_bot && !row.owner_id.is_empty()),
            move || {
                let open = move || i18n.tr("Open app", "打开应用");
                view()
                    .flex_none()
                    .tooltip(ui::hint(move || open().to_owned()))
                    .child(ui::glass_button(
                        ui::round_button()
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
                            .child(ui::icon(ui::Icon::Bot)),
                    ))
            },
        ))
        .child(panel_toggle(
            navigation,
            ui::Icon::Search,
            Key::SearchMessages,
            Panel::Search,
        ))
        .child(panel_toggle(
            navigation,
            ui::Icon::InfoCircle,
            Key::ChatInfo,
            Panel::Info,
        ))
}

fn panel_toggle(
    navigation: ChatNavigation,
    icon: ui::Icon,
    label: Key,
    mode: Panel,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::glass_button(
        ui::round_button()
            .accessibility_label(move || Some(i18n.text(label).into()))
            .selected(move || navigation.panel.get() == Some(mode))
            .on_press(move || navigation.toggle_panel(mode))
            .child(ui::icon(icon)),
    )
}
