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
    width: Signal<f32>,
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
    let layout = computed(move || {
        let leading = if narrow() {
            ui::title_bar_leading(rail.width()).0
        } else {
            ui::space_lg().0
        };
        let bot = row.with(|row| !group && row.is_bot && !row.owner_id.is_empty());
        // Reserve fixed controls and their gaps before sharing the remaining
        // space. The title can truncate, but never takes the pin's whole slot.
        let controls = 2.0 + f32::from(bot) + f32::from(narrow());
        let gaps = 3.0 + f32::from(bot) + f32::from(narrow());
        let available = (width.get()
            - leading
            - ui::space_md().0
            - controls * ui::glass_bar_height().0
            - gaps * ui::space_sm().0)
            .max(0.0);
        let identity = (available * 0.36).min(ui::card_width().0);
        let inline = available - identity >= ui::glass_inline_pin_width().0;
        let identity = if inline {
            identity
        } else {
            // The compact pin needs just one button; give its unused summary
            // space back to the conversation title.
            (available - ui::glass_bar_height().0 - ui::space_sm().0)
                .max(0.0)
                .min(ui::card_width().0)
        };
        (px(identity), inline)
    });
    let inline_pin = computed(move || layout.get().1);
    let compact_pin = computed(move || !inline_pin.get());
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
            ui::glass_pill()
                .max_w(move || layout.get().0)
                .min_w_0()
                .flex_shrink(1.0)
                .child(
                    ui::nav_row()
                        .min_w_0()
                        .min_h_0()
                        .h_full()
                        .flex_shrink(1.0)
                        .rounded(ui::glass_radius())
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
        // The center flexes between the bounded identity and fixed actions.
        .child(
            view()
                .flex_1()
                .min_w_0()
                .flex_row()
                .justify_center()
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
        .child(
            view()
                .flex_none()
                .flex_row()
                .items_center()
                .child(pinned.compact_button(compact_pin))
                .child(panel_toggle(
                    navigation,
                    ui::Icon::Search,
                    Key::SearchMessages,
                    Panel::Search,
                )),
        )
        .child(
            view()
                .flex_none()
                .tooltip(
                    ui::hint(move || i18n.tr("Chat details", "聊天详情").to_owned())
                        .disabled(move || navigation.panel.get() == Some(Panel::Info)),
                )
                .child(ui::glass_button(
                    ui::round_button()
                        .accessibility_label(move || Some(i18n.tr("More", "更多").into()))
                        .selected(move || navigation.panel.get() == Some(Panel::Info))
                        .on_press(move || navigation.toggle_panel(Panel::Info))
                        .child(ui::icon(ui::Icon::More)),
                )),
        )
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
