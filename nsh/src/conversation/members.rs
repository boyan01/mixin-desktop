use std::collections::HashSet;

use mixin_desktop_api::{ConversationParticipantItem, UserProfileItem};

use super::*;
use crate::{motion::Presence, navigation::ChatNavigation};

const PAGE_SIZE: i64 = 100;
const MAX_MEMBERS: usize = 1024;

pub(super) fn screen(state: State) -> impl IntoElement {
    let navigation =
        use_context::<ChatNavigation>().expect("The signed-in session provides navigation");
    let query = signal(String::new());
    let adding = signal(false);
    let profile = signal_with(None::<ConversationParticipantItem>, SignalEquals::Never);
    let adding_presence = Presence::modal(move || adding.get());
    let profile_presence = Presence::modal(move || profile.with(Option::is_some));
    let filtered = computed(move || {
        let keyword = query.get().trim().to_lowercase();
        state.members.with(|members| {
            members
                .iter()
                .enumerate()
                .filter(|(_, member)| {
                    keyword.is_empty()
                        || member.full_name.to_lowercase().contains(&keyword)
                        || member.identity_number.contains(&keyword)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>()
        })
    });
    view()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .p(ui::space_lg())
                .gap(ui::space_sm())
                .child(
                    ui::search_pill().flex_none().child(
                        ui::search_input()
                            .label(move || tr("Search participants", "搜索群成员"))
                            .placeholder(move || tr("Search", "搜索").to_owned())
                            .value(move || query.get())
                            .on_change(move |text: &str| query.set(text.into())),
                    ),
                )
                .child(when(
                    move || state.can_manage(),
                    move || {
                        ui::action(
                            move || tr("Add participants…", "添加群成员…"),
                            move || adding.set(true),
                        )
                        .disabled(state.busy)
                    },
                )),
        )
        .child(
            virtual_list(
                move || filtered.with(Vec::len),
                move |at| {
                    let member = move || {
                        filtered
                            .with(|indices| indices.get(at.index()).copied())
                            .and_then(|index| {
                                state.members.with(|members| members.get(index).cloned())
                            })
                    };
                    member_row(state, navigation, profile, member)
                },
            )
            .keys(move || {
                filtered.with(|indices| {
                    state.members.with(|members| {
                        indices
                            .iter()
                            .filter_map(|index| {
                                members.get(*index).map(|member| member.user_id.clone())
                            })
                            .collect::<Vec<_>>()
                    })
                })
            })
            .estimated_item_size(parts::row_height())
            .flex_1()
            .min_h_0()
            .style_when(move || filtered.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || filtered.with(Vec::is_empty) && !state.loading.get(),
            || ui::placeholder(move || tr("No participants found", "未找到群成员")),
        ))
        .child(when(
            move || adding_presence.mounted(),
            move || add_form(state, adding, adding_presence),
        ))
        .child(when(
            move || profile_presence.mounted(),
            move || profile_dialog(state, profile, profile_presence),
        ))
}

fn member_row(
    state: State,
    navigation: ChatNavigation,
    profile: Signal<Option<ConversationParticipantItem>>,
    member: impl Fn() -> Option<ConversationParticipantItem> + Copy + 'static,
) -> impl IntoElement {
    let other = move || member().is_some_and(|member| member.user_id != state.own_id());
    let owner = move || state.role().as_deref() == Some("OWNER");
    ui::menu()
        .trigger(member_summary(member))
        .child(
            ui::menu_row(move || tr("Profile", "个人资料"))
                .on_select(move || profile.set(member())),
        )
        .child(
            ui::menu_row(move || tr("Message", "发消息")).on_select(move || {
                if let Some(member) = member() {
                    navigation.open_user(state.client(), member.user_id, state.failed);
                }
            }),
        )
        .child(when(
            move || owner() && other(),
            move || {
                ui::menu_row(move || {
                    if member().is_some_and(|member| member.role.as_deref() == Some("ADMIN")) {
                        tr("Dismiss as admin", "取消管理员")
                    } else {
                        tr("Make group admin", "设为管理员")
                    }
                })
                .disabled(state.busy)
                .on_select(move || {
                    if let Some(member) = member() {
                        toggle_admin(state, member);
                    }
                })
            },
        ))
        .child(when(
            move || {
                other()
                    && (owner()
                        || (state.can_manage()
                            && member().is_some_and(|member| member.role.is_none())))
            },
            move || {
                ui::menu_row(move || tr("Remove from group…", "移出群组…"))
                    .text_color(ui::danger)
                    .disabled(state.busy)
                    .on_select(move || {
                        if let Some(member) = member() {
                            remove_member(state, member);
                        }
                    })
            },
        ))
}

fn member_summary(
    member: impl Fn() -> Option<ConversationParticipantItem> + Copy + 'static,
) -> nsh_base::ButtonElement {
    parts::compact_row()
        .child(ui::avatar_view(
            move || member().map(|member| member.user_id).unwrap_or_default(),
            move || {
                member()
                    .map(|member| member.full_name.into())
                    .unwrap_or_default()
            },
            move || member().map(|member| member.avatar_url).unwrap_or_default(),
            ui::AvatarSize::Large,
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap(ui::space_xs())
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap(ui::space_xs())
                        .min_w_0()
                        .child(
                            text(move || {
                                member().map(|member| member.full_name).unwrap_or_default()
                            })
                            .text_role(ui::TextRole::Label)
                            .number_of_lines(1)
                            .flex_1()
                            .min_w_0(),
                        )
                        .child(dynamic(move || {
                            let value = member();
                            untracked(|| match value {
                                Some(value) => crate::appearance::identity_badge(
                                    false,
                                    value.is_verified,
                                    value.is_bot,
                                    value.membership.as_deref(),
                                ),
                                None => view().hidden().into_element(),
                            })
                        })),
                )
                .child(
                    text(move || {
                        member()
                            .map(|member| member.identity_number)
                            .unwrap_or_default()
                    })
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::fg_secondary),
                ),
        )
        .child(
            text(move || {
                member()
                    .map(|member| match member.role.as_deref() {
                        Some("OWNER") => tr("Owner", "群主"),
                        Some("ADMIN") => tr("Admin", "管理员"),
                        _ => "",
                    })
                    .unwrap_or_default()
            })
            .text_role(ui::TextRole::Caption)
            .text_color(ui::fg_secondary),
        )
}

fn toggle_admin(state: State, member: ConversationParticipantItem) {
    let role = (member.role.as_deref() != Some("ADMIN")).then(|| "ADMIN".into());
    let client = state.client();
    let id = state.id();
    state.submit(
        async move {
            client
                .conversation()
                .update_participants(id, "ROLE".into(), vec![member.user_id], role)
                .await
        },
        || {},
    );
}

fn remove_member(state: State, member: ConversationParticipantItem) {
    let client = state.client();
    let id = state.id();
    state.confirm_then(
        Prompt::new(
            tr("Remove participant?", "移除群成员？"),
            member.full_name,
            tr("Remove", "移除"),
        ),
        move || async move {
            client
                .conversation()
                .update_participants(id, "REMOVE".into(), vec![member.user_id], None)
                .await
        },
    );
}

fn profile_dialog(
    state: State,
    profile: Signal<Option<ConversationParticipantItem>>,
    presence: Presence,
) -> impl IntoElement {
    let id = profile.with_untracked(|profile| {
        profile
            .as_ref()
            .map(|profile| profile.user_id.clone())
            .unwrap_or_default()
    });
    ui::dialog(
        presence,
        move || profile.set(None),
        ui::dialog_card(ui::DialogWidth::Compact)
            .p_0()
            .gap_0()
            .overflow_hidden()
            .child(super::profile(state.client(), id, move || {
                profile.set(None)
            })),
    )
}

fn add_form(state: State, open: Signal<bool>, presence: Presence) -> impl IntoElement {
    let users = signal_with(Vec::<UserProfileItem>::new(), SignalEquals::Never);
    let selected = signal(HashSet::<String>::new());
    let query = signal(String::new());
    let loading = signal(true);
    let failed = signal(false);
    let revision = signal(0_u64);
    let page = signal(0_i64);
    let has_next = signal(false);
    effect(move || {
        let _revision = revision.get();
        let keyword = query.get();
        let offset = page.get() * PAGE_SIZE;
        let client = state.client();
        let conversation = state.id();
        batch(|| {
            loading.set(true);
            failed.set(false);
        });
        spawn(async move {
            if !keyword.trim().is_empty() {
                timer(parts::SEARCH_DELAY).await;
            }
            // The query already excludes participants and anyone who is not a friend.
            let result = runtime::run(async move {
                client
                    .user()
                    .invitable_users_page(conversation, keyword, offset, PAGE_SIZE + 1)
                    .await
            })
            .await;
            match result {
                Ok(mut rows) => batch(|| {
                    has_next.set(rows.len() > PAGE_SIZE as usize);
                    rows.truncate(PAGE_SIZE as usize);
                    users.set(rows);
                }),
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    let close = move || {
        if !state.busy.get_untracked() {
            open.set(false);
        }
    };
    ui::dialog(
        presence,
        close,
        parts::tall_dialog_card()
            .gap(ui::space_md())
            .child(ui::dialog_title(move || {
                tr("Add participants", "添加群成员")
            }))
            .child(
                ui::search_pill().flex_none().child(
                    ui::search_input()
                        .label(move || tr("Search contacts", "搜索联系人"))
                        .placeholder(move || tr("Search contacts", "搜索联系人").to_owned())
                        .value(move || query.get())
                        .on_change(move |text: &str| {
                            batch(|| {
                                page.set(0);
                                query.set(text.into());
                            })
                        }),
                ),
            )
            .child(when(
                move || loading.get() && users.with(Vec::is_empty),
                || parts::status(move || tr("Loading…", "加载中…")),
            ))
            .child(when(failed, move || {
                parts::failure(
                    move || tr("Couldn’t load. Try again.", "加载失败，请重试。"),
                    move || revision.update(|value| *value += 1),
                )
            }))
            .child(
                virtual_list(
                    move || users.with(Vec::len),
                    move |at| {
                        let user = move || users.with(|users| users.get(at.index()).cloned());
                        let chosen = move || {
                            user().is_some_and(|user| {
                                selected.with(|ids| ids.contains(&user.user_id))
                            })
                        };
                        parts::entity_row(
                            move || user().map(|user| user.user_id).unwrap_or_default(),
                            move || user().map(|user| user.full_name).unwrap_or_default(),
                            move || user().map(|user| user.avatar_url).unwrap_or_default(),
                            move || user().map(|user| user.identity_number).unwrap_or_default(),
                            chosen,
                        )
                        .disabled(state.busy)
                        .on_press(move || {
                            if let Some(user) = user() {
                                selected.update(|ids| {
                                    if !ids.remove(&user.user_id)
                                        && ids.len() + state.members.with_untracked(Vec::len)
                                            < MAX_MEMBERS
                                    {
                                        ids.insert(user.user_id);
                                    }
                                });
                            }
                        })
                    },
                )
                .keys(move || {
                    users.with(|users| {
                        users
                            .iter()
                            .map(|user| user.user_id.clone())
                            .collect::<Vec<_>>()
                    })
                })
                .estimated_item_size(parts::row_height())
                .flex_1()
                .min_h_0()
                .style_when(move || users.with(Vec::is_empty), |style| style.hidden()),
            )
            .child(when(
                move || !loading.get() && !failed.get() && users.with(Vec::is_empty),
                || ui::placeholder(move || tr("No available contacts", "没有可添加的联系人")),
            ))
            .child(parts::pager(
                page,
                move || has_next.get(),
                move || loading.get() || state.busy.get(),
            ))
            .child(
                parts::dialog_actions()
                    .child(ui::action(move || tr("Cancel", "取消"), close).disabled(state.busy))
                    .child(
                        ui::primary()
                            .disabled(move || {
                                state.busy.get()
                                    || selected.with(HashSet::is_empty)
                                    || !state.can_manage()
                            })
                            .on_press(move || {
                                if !presence.interactive() {
                                    return;
                                }
                                let ids = selected
                                    .with_untracked(|ids| ids.iter().cloned().collect::<Vec<_>>());
                                let client = state.client();
                                let id = state.id();
                                state.submit(
                                    async move {
                                        client
                                            .conversation()
                                            .update_participants(id, "ADD".into(), ids, None)
                                            .await
                                    },
                                    move || open.set(false),
                                );
                            })
                            .child(text(move || {
                                format!("{} ({})", tr("Add", "添加"), selected.with(HashSet::len))
                            })),
                    ),
            ),
    )
}
