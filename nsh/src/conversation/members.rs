use std::collections::HashSet;

use mixin_desktop_api::{ConversationParticipantItem, UserProfileItem};
use nsh_base::menu_item;

use super::*;

pub(super) fn screen(state: State) -> impl IntoElement {
    let navigation = use_context::<crate::navigation::ChatNavigation>()
        .expect("The signed-in session provides navigation");
    let query = signal(String::new());
    let adding = signal(false);
    let profile = signal_with(None::<ConversationParticipantItem>, SignalEquals::Never);
    let adding_presence = crate::motion::Presence::modal(move || adding.get());
    let profile_presence = crate::motion::Presence::modal(move || profile.with(Option::is_some));
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
                .px_4()
                .pb_3()
                .gap_2()
                .child(
                    ui::field()
                        .label(move || tr("Search participants", "搜索群成员"))
                        .placeholder(move || tr("Search", "搜索").to_owned())
                        .value(move || query.get())
                        .on_change(move |text: &str| query.set(text.into())),
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
                    ui::menu()
                        .trigger(
                            ui::command()
                                .justify_start()
                                .min_h_16()
                                .px_4()
                                .gap_3()
                                .child(ui::avatar_view(
                                    move || {
                                        member()
                                            .map(|member| member.full_name.into())
                                            .unwrap_or_default()
                                    },
                                    move || {
                                        member().map(|member| member.avatar_url).unwrap_or_default()
                                    },
                                ))
                                .child(
                                    view()
                                        .flex_1()
                                        .min_w_0()
                                        .gap_1()
                                        .child(
                                            view()
                                                .flex_row()
                                                .items_center()
                                                .gap_1()
                                                .min_w_0()
                                                .child(
                                                    text(move || {
                                                        member()
                                                            .map(|member| member.full_name)
                                                            .unwrap_or_default()
                                                    })
                                                    .text_sm()
                                                    .number_of_lines(1)
                                                    .flex_1()
                                                    .min_w_0(),
                                                )
                                                .child(dynamic(move || {
                                                    let value = member();
                                                    untracked(|| match value {
                                                        Some(value) => {
                                                            crate::appearance::identity_badge(
                                                                false,
                                                                value.is_verified,
                                                                value.is_bot,
                                                                value.membership.as_deref(),
                                                            )
                                                        }
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
                                            .text_xs()
                                            .text_color(move || theme().muted_foreground),
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
                                    .text_xs()
                                    .text_color(move || theme().muted_foreground),
                                ),
                        )
                        .child(
                            menu_item()
                                .px_3()
                                .py_2()
                                .hover(|style| style.bg(move || theme().accent))
                                .child(text(move || tr("Profile", "个人资料")))
                                .on_select(move || profile.set(member())),
                        )
                        .child(
                            menu_item()
                                .px_3()
                                .py_2()
                                .hover(|style| style.bg(move || theme().accent))
                                .child(text(move || tr("Message", "发消息")))
                                .on_select(move || {
                                    if let Some(member) = member() {
                                        navigation.open_user(
                                            state.client(),
                                            member.user_id,
                                            state.failed,
                                        );
                                    }
                                }),
                        )
                        .child(when(
                            move || {
                                state.role().as_deref() == Some("OWNER")
                                    && member().is_some_and(|member| {
                                        member.user_id
                                            != state.account.with(|account| account.account_id())
                                    })
                            },
                            move || {
                                menu_item()
                                    .px_3()
                                    .py_2()
                                    .hover(|style| style.bg(move || theme().accent))
                                    .disabled(state.busy)
                                    .child(text(move || {
                                        if member().is_some_and(|member| {
                                            member.role.as_deref() == Some("ADMIN")
                                        }) {
                                            tr("Dismiss as admin", "取消管理员")
                                        } else {
                                            tr("Make group admin", "设为管理员")
                                        }
                                    }))
                                    .on_select(move || {
                                        if let Some(member) = member() {
                                            let role = (member.role.as_deref() != Some("ADMIN"))
                                                .then(|| "ADMIN".into());
                                            let client = state.client();
                                            let id = state.id();
                                            state.submit(
                                                async move {
                                                    client
                                                        .conversation()
                                                        .update_participants(
                                                            id,
                                                            "ROLE".into(),
                                                            vec![member.user_id],
                                                            role,
                                                        )
                                                        .await
                                                },
                                                || {},
                                            );
                                        }
                                    })
                            },
                        ))
                        .child(when(
                            move || {
                                member().is_some_and(|member| {
                                    let own_id = state.account.with(|account| account.account_id());
                                    member.user_id != own_id
                                        && (state.role().as_deref() == Some("OWNER")
                                            || (state.can_manage() && member.role.is_none()))
                                })
                            },
                            move || {
                                menu_item()
                                    .px_3()
                                    .py_2()
                                    .text_color(move || theme().destructive)
                                    .hover(|style| style.bg(move || theme().accent))
                                    .disabled(state.busy)
                                    .child(text(move || tr("Remove from group…", "移出群组…")))
                                    .on_select(move || {
                                        if let Some(member) = member() {
                                            spawn(async move {
                                                if confirm(
                                                    tr("Remove participant?", "移除群成员？"),
                                                    member.full_name,
                                                    tr("Remove", "移除"),
                                                )
                                                .await
                                                {
                                                    let client = state.client();
                                                    let id = state.id();
                                                    state.submit(
                                                        async move {
                                                            client
                                                                .conversation()
                                                                .update_participants(
                                                                    id,
                                                                    "REMOVE".into(),
                                                                    vec![member.user_id],
                                                                    None,
                                                                )
                                                                .await
                                                        },
                                                        || {},
                                                    );
                                                }
                                            });
                                        }
                                    })
                            },
                        ))
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
            .estimated_item_size(px(64.0))
            .flex_1()
            .min_h_0(),
        )
        .child(when(
            move || filtered.with(Vec::is_empty) && !state.loading.get(),
            || {
                view().p_4().child(
                    text(move || tr("No participants found", "未找到群成员"))
                        .text_sm()
                        .text_color(move || theme().muted_foreground),
                )
            },
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

fn profile_dialog(
    state: State,
    profile: Signal<Option<ConversationParticipantItem>>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let id = profile.with_untracked(|profile| {
        profile
            .as_ref()
            .map(|profile| profile.user_id.clone())
            .unwrap_or_default()
    });
    ui::modal(
        presence,
        move || profile.set(None),
        view()
            .w_96()
            .max_w(relative(1.0))
            .h(px(640.0))
            .max_h(relative(0.9))
            .rounded_xl()
            .bg(move || theme().popover)
            .child(super::profile(state.client(), id, move || {
                profile.set(None)
            })),
    )
}

fn add_form(
    state: State,
    open: Signal<bool>,
    presence: crate::motion::Presence,
) -> impl IntoElement {
    let users = signal_with(Vec::<UserProfileItem>::new(), SignalEquals::Never);
    let selected = signal(HashSet::<String>::new());
    let query = signal(String::new());
    let loading = signal(true);
    let failed = signal(false);
    let revision = signal(0_u64);
    let page = signal(0_i64);
    let has_next = signal(false);
    let owner = Owner::current();
    let request = signal(0_u64);
    effect(move || {
        let _revision = revision.get();
        let keyword = query.get();
        let page = page.get();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        let client = state.client();
        let conversation = state.id();
        batch(|| {
            loading.set(true);
            failed.set(false);
            users.set(Vec::new());
            has_next.set(false);
        });
        spawn(async move {
            let result = runtime::run(async move {
                client
                    .user()
                    .invitable_users_page(conversation, keyword, page * 100, 101)
                    .await
            })
            .await;
            if !owner.is_live() || request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok(mut rows) => batch(|| {
                    has_next.set(rows.len() > 100);
                    rows.truncate(100);
                    users.set(rows);
                }),
                Err(_) => failed.set(true),
            }
            loading.set(false);
        });
    });
    let candidates = computed(move || {
        let keyword = query.get().trim().to_lowercase();
        let existing = state.members.with(|members| {
            members
                .iter()
                .map(|member| member.user_id.clone())
                .collect::<HashSet<_>>()
        });
        users.with(|users| {
            users
                .iter()
                .enumerate()
                .filter(|(_, user)| {
                    !existing.contains(&user.user_id)
                        && user.relationship == "FRIEND"
                        && (keyword.is_empty()
                            || user.full_name.to_lowercase().contains(&keyword)
                            || user.identity_number.contains(&keyword))
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>()
        })
    });
    ui::modal(
        presence,
        move || {
            if !state.busy.get_untracked() {
                open.set(false);
            }
        },
        view()
            .w_96()
            .max_w(relative(1.0))
            .h(px(560.0))
            .max_h(relative(0.9))
            .rounded_xl()
            .bg(move || theme().popover)
            .p_4()
            .gap_3()
            .child(
                text(move || tr("Add participants", "添加群成员"))
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                ui::field()
                    .label(move || tr("Search contacts", "搜索联系人"))
                    .placeholder(move || tr("Search contacts", "搜索联系人").to_owned())
                    .value(move || query.get())
                    .on_change(move |text: &str| {
                        batch(|| {
                            page.set(0);
                            query.set(text.into());
                        })
                    }),
            )
            .child(when(loading, || {
                text(move || tr("Loading…", "加载中…")).text_sm()
            }))
            .child(when(failed, move || {
                ui::action(
                    move || tr("Retry", "重试"),
                    move || revision.update(|value| *value += 1),
                )
            }))
            .child(
                virtual_list(
                    move || candidates.with(Vec::len),
                    move |at| {
                        let user = move || {
                            candidates
                                .with(|indices| indices.get(at.index()).copied())
                                .and_then(|index| users.with(|users| users.get(index).cloned()))
                        };
                        ui::command()
                            .justify_start()
                            .min_h_14()
                            .gap_3()
                            .selected(move || {
                                user().is_some_and(|user| {
                                    selected.with(|ids| ids.contains(&user.user_id))
                                })
                            })
                            .selected_style(|style| style.bg(move || theme().accent))
                            .disabled(move || state.busy.get() || loading.get())
                            .on_press(move || {
                                if let Some(user) = user() {
                                    selected.update(|ids| {
                                        if !ids.remove(&user.user_id)
                                            && ids.len() + state.members.with_untracked(Vec::len)
                                                < 1024
                                        {
                                            ids.insert(user.user_id);
                                        }
                                    });
                                }
                            })
                            .child(ui::avatar_view(
                                move || {
                                    user().map(|user| user.full_name.into()).unwrap_or_default()
                                },
                                move || user().map(|user| user.avatar_url).unwrap_or_default(),
                            ))
                            .child(
                                text(move || user().map(|user| user.full_name).unwrap_or_default())
                                    .text_sm()
                                    .number_of_lines(1),
                            )
                    },
                )
                .keys(move || {
                    candidates.with(|indices| {
                        users.with(|users| {
                            indices
                                .iter()
                                .filter_map(|index| {
                                    users.get(*index).map(|user| user.user_id.clone())
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                })
                .estimated_item_size(px(56.0))
                .flex_1()
                .min_h_0(),
            )
            .child(when(
                move || !loading.get() && !failed.get() && candidates.with(Vec::is_empty),
                || {
                    text(move || tr("No available contacts", "没有可添加的联系人"))
                        .text_sm()
                        .text_color(move || theme().muted_foreground)
                },
            ))
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(
                        ui::action(
                            move || tr("Previous page", "上一页"),
                            move || page.update(|value| *value = value.saturating_sub(1)),
                        )
                        .disabled(move || loading.get() || page.get() == 0 || state.busy.get()),
                    )
                    .child(view().flex_1())
                    .child(
                        ui::action(
                            move || tr("Next page", "下一页"),
                            move || page.update(|value| *value = value.saturating_add(1)),
                        )
                        .disabled(move || loading.get() || !has_next.get() || state.busy.get()),
                    ),
            )
            .child(
                view()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        ui::action(move || tr("Cancel", "取消"), move || open.set(false))
                            .disabled(state.busy),
                    )
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
