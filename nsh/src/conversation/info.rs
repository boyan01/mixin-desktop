use nsh_base::SelectItem;

use super::*;

fn exited_group(state: State) -> bool {
    state.row.with(|row| {
        row.as_ref()
            .is_some_and(|row| row.category == "GROUP" && row.status == 3)
    })
}

fn muted(state: State) -> bool {
    state
        .row
        .with(|row| row.as_ref().is_some_and(|row| row.is_muted))
}

fn row_text(state: State, read: fn(&ConversationListData) -> String) -> String {
    state
        .row
        .with(|row| row.as_ref().map(read).unwrap_or_default())
}

fn detail_url(state: State) -> String {
    state.detail.with(|detail| {
        detail
            .as_ref()
            .map(|detail| detail.code_url.clone())
            .unwrap_or_default()
    })
}

pub(super) fn screen(state: State) -> impl IntoElement {
    scroll().flex_1().min_h_0().child(
        view()
            .pb_6()
            .child(identity(state))
            .child(when(
                move || state.group() && !exited_group(state),
                move || group_rows(state),
            ))
            .child(when(
                move || !state.group(),
                move || {
                    row(
                        move || tr("Profile", "个人资料"),
                        move || state.page.set(Page::Profile),
                    )
                },
            ))
            .child(section(move || tr("Chats", "聊天")))
            .child(chat_rows(state))
            .child(section(move || tr("History", "聊天记录")))
            .child(history_rows(state))
            .child(when(
                move || state.group(),
                move || {
                    text(move || {
                        state.detail.with(|detail| {
                            detail
                                .as_ref()
                                .map(|detail| {
                                    let date = crate::presentation::date(
                                        i18n::current(),
                                        detail.created_at_millis,
                                    );
                                    i18n::current().format(
                                        "Created {arg0}",
                                        "创建于 {arg0}",
                                        &[("arg0", &date)],
                                    )
                                })
                                .unwrap_or_default()
                        })
                    })
                    .text_xs()
                    .text_center()
                    .py_4()
                    .text_color(move || theme().muted_foreground)
                },
            )),
    )
}

fn identity(state: State) -> impl IntoElement {
    let description = move || {
        if state.group() {
            state.detail.with(|detail| {
                detail
                    .as_ref()
                    .map(|detail| detail.announcement.clone())
                    .unwrap_or_default()
            })
        } else {
            state.biography.get()
        }
    };
    view()
        .p_4()
        .gap_2()
        .items_center()
        .child(
            ui::avatar_view(
                move || row_text(state, |row| row.name.clone()).into(),
                move || row_text(state, |row| row.avatar_url.clone()),
            )
            .size_16(),
        )
        .child(
            text(move || row_text(state, |row| row.name.clone()))
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_center(),
        )
        .child(
            text(move || {
                row_text(state, |row| {
                    if row.category == "GROUP" {
                        i18n::current().members(row.participant_count)
                    } else {
                        row.identity_number.clone()
                    }
                })
            })
            .text_sm()
            .text_color(move || theme().muted_foreground),
        )
        .child(
            text(description)
                .text_sm()
                .text_center()
                .style_when(move || description().is_empty(), |style| style.hidden()),
        )
}

fn group_rows(state: State) -> impl IntoElement {
    view()
        .child(row(
            move || tr("Group participants", "群组成员"),
            move || state.page.set(Page::Members),
        ))
        .child(when(
            move || state.can_manage(),
            move || {
                row(
                    move || tr("Edit group…", "编辑群组…"),
                    move || state.page.set(Page::Edit),
                )
            },
        ))
        .child(when(
            move || state.can_manage(),
            move || {
                row(
                    move || tr("Invite to group…", "邀请加入群组…"),
                    move || state.page.set(Page::Invite),
                )
            },
        ))
}

fn chat_rows(state: State) -> impl IntoElement {
    let navigation = use_context::<crate::navigation::ChatNavigation>()
        .expect("The signed-in session provides navigation");
    view()
        .child(row(
            move || tr("Shared media", "共享媒体"),
            move || state.page.set(Page::Shared),
        ))
        .child(when(
            move || state.shared_apps_preview.with(|apps| !apps.is_empty()),
            move || shared_apps_row(state),
        ))
        .child(row(
            move || tr("Search messages", "搜索消息"),
            move || navigation.open_search_from_info(),
        ))
        .child(row(
            move || tr("Circles", "圈子"),
            move || state.page.set(Page::Circles),
        ))
        .child(when(
            move || !exited_group(state),
            move || view().child(expiry_row(state)).child(mute_row(state)),
        ))
        .child(when(
            move || {
                !state.group()
                    && state.row.with(|row| {
                        row.as_ref()
                            .is_some_and(|row| row.owner_id != state.own_id())
                    })
            },
            move || {
                row(
                    move || tr("Edit name…", "编辑名称…"),
                    move || state.page.set(Page::EditName),
                )
            },
        ))
}

fn shared_apps_row(state: State) -> impl IntoElement {
    row(
        move || tr("Shared apps", "共享应用"),
        move || state.page.set(Page::Apps),
    )
    .gap_3()
    .child(view().flex_1())
    .child(dynamic(move || {
        let apps = state.shared_apps_preview.get();
        let width = 28.0 + apps.len().saturating_sub(1) as f32 * 14.0;
        view()
            .relative()
            .h_7()
            .w(px(width))
            .flex_shrink_0()
            .children(apps.into_iter().enumerate().rev().map(|(index, app)| {
                ui::avatar_view(
                    move || app.name.clone().into(),
                    move || app.icon_url.clone(),
                )
                .absolute()
                .top_0()
                .left(px(index as f32 * 14.0))
                .size_7()
                .border_2()
                .border_color(move || theme().background)
            }))
    }))
}

fn expiry_row(state: State) -> impl IntoElement {
    row(
        move || tr("Disappearing messages", "阅后即焚"),
        move || state.page.set(Page::Expiry),
    )
    .disabled(move || state.group() && !state.can_manage())
    .child(view().flex_1())
    .child(
        text(move || {
            let seconds = state
                .detail
                .with(|detail| detail.as_ref().map_or(0, |detail| detail.expire_in));
            if seconds > 0 {
                crate::presentation::system_message::expire_duration(i18n::current(), seconds)
            } else {
                tr("Off", "关闭").to_owned()
            }
        })
        .text_sm()
        .text_color(move || theme().muted_foreground),
    )
}

fn mute_row(state: State) -> impl IntoElement {
    row(
        move || {
            if muted(state) {
                tr("Unmute", "取消静音")
            } else {
                tr("Mute…", "静音…")
            }
        },
        move || {
            if state
                .row
                .with_untracked(|row| row.as_ref().is_some_and(|row| row.is_muted))
            {
                set_mute(state, 0);
            } else {
                choose_mute(state);
            }
        },
    )
    .disabled(state.busy)
    .flex_col()
    .items_start()
    .gap_1()
    .child(when(
        move || muted(state),
        move || {
            text(move || {
                let millis = state
                    .detail
                    .with(|detail| detail.as_ref().map_or(0, |detail| detail.mute_until_millis));
                chrono::DateTime::from_timestamp_millis(millis)
                    .filter(|_| millis > 0)
                    .map(|date| {
                        format!(
                            "{}, {}",
                            crate::presentation::date(i18n::current(), millis),
                            date.with_timezone(&chrono::Local).format("%H:%M")
                        )
                    })
                    .unwrap_or_default()
            })
            .text_sm()
            .text_color(move || theme().muted_foreground)
        },
    ))
}

fn set_mute(state: State, duration: i64) {
    let Some(value) = state.row.get_untracked() else {
        return;
    };
    let client = state.client();
    state.submit(
        async move {
            client
                .conversation()
                .set_muted(
                    value.conversation_id,
                    value.owner_id,
                    value.category,
                    duration,
                )
                .await
        },
        || {},
    );
}

fn choose_mute(state: State) {
    state.owner.run(|| {
        spawn(async move {
            let answer = alert_dialog()
                .title(tr("Mute notifications", "消息免打扰"))
                .action("hour", tr("1 hour", "1 小时"))
                .action("eight-hours", tr("8 hours", "8 小时"))
                .action("week", tr("1 week", "1 周"))
                .action("year", tr("1 year", "1 年"))
                .cancel("cancel", tr("Cancel", "取消"))
                .show()
                .await;
            let duration = match answer.ok().and_then(|answer| answer.action).as_deref() {
                Some("hour") => 3600,
                Some("eight-hours") => 8 * 3600,
                Some("week") => 7 * 86400,
                Some("year") => 365 * 86400,
                _ => return,
            };
            set_mute(state, duration);
        })
    });
}

fn history_rows(state: State) -> impl IntoElement {
    view()
        .child(
            row(
                move || tr("Clear chat…", "清空聊天记录…"),
                move || {
                    let client = state.client();
                    let id = state.id();
                    state.confirm_then(
                        Prompt::new(
                            tr("Clear chat?", "清空聊天记录？"),
                            tr(
                                "Messages in this chat will be removed from this device.",
                                "将从此设备移除这段聊天的消息。",
                            ),
                            tr("Clear", "清空"),
                        ),
                        move || async move { client.conversation().clear_conversation(id).await },
                    );
                },
            )
            .disabled(state.busy)
            .text_color(move || theme().destructive),
        )
        .child(when(
            move || state.group(),
            move || {
                row(
                    move || {
                        if exited_group(state) {
                            tr("Delete group…", "删除群组…")
                        } else {
                            tr("Exit group…", "退出群组…")
                        }
                    },
                    move || leave_group(state),
                )
                .disabled(state.busy)
                .text_color(move || theme().destructive)
            },
        ))
}

fn leave_group(state: State) {
    let client = state.client();
    let id = state.id();
    if untracked(|| exited_group(state)) {
        state.confirm_then_finish(
            Prompt::new(
                tr("Delete group?", "删除群组？"),
                tr(
                    "This removes the chat and its message history from this device.",
                    "此操作将从此设备删除会话及其聊天记录。",
                ),
                tr("Delete", "删除"),
            ),
            move || async move { client.conversation().delete_conversation(id).await },
            move || state.close(),
        );
    } else {
        state.confirm_then(
            Prompt::new(
                tr("Exit group?", "退出群组？"),
                tr(
                    "You will leave this group and remove its local conversation.",
                    "你将退出此群组并移除本地会话。",
                ),
                tr("Exit", "退出"),
            ),
            move || async move { client.conversation().exit_group(id).await },
        );
    }
}

pub(super) fn edit(state: State) -> impl IntoElement {
    let avatar = crate::avatar::Picker::new();
    let (initial_name, initial_description) = state.detail.with_untracked(|detail| {
        detail
            .as_ref()
            .map(|detail| (detail.name.clone(), detail.announcement.clone()))
            .unwrap_or_default()
    });
    let (draft_name, draft_description) = state
        .edit_draft
        .with(|draft| draft.clone())
        .unwrap_or_else(|| (initial_name.clone(), initial_description.clone()));
    let name = signal(draft_name);
    let description = signal(draft_description);
    let initial_name = stored(initial_name);
    let initial_description = stored(initial_description);
    let keep_draft = move || {
        state
            .edit_draft
            .replace(Some((name.get_untracked(), description.get_untracked())));
    };
    scroll().flex_1().min_h_0().child(
        view()
            .p_4()
            .gap_4()
            .child(
                ui::action(
                    move || tr("Change group photo…", "更改群头像…"),
                    move || {
                        spawn(async move {
                            state.failed.set(false);
                            if avatar.pick().await.is_err() {
                                state.failed.set(true);
                            }
                        });
                    },
                )
                .disabled(move || state.busy.get() || !state.can_manage() || avatar.busy()),
            )
            .child(avatar.surface(true, move |data| {
                let client = state.client();
                let id = state.id();
                state.submit(
                    async move {
                        client
                            .conversation()
                            .update_conversation_avatar(id, data)
                            .await
                    },
                    || {},
                );
            }))
            .child(text(move || tr("Name", "名称")).text_sm())
            .child(
                ui::field()
                    .label(move || tr("Name", "名称"))
                    .value(move || name.get())
                    .max_length(40)
                    .on_change(move |value: &str| {
                        name.set(value.into());
                        keep_draft();
                    }),
            )
            .child(text(move || tr("Description", "群公告")).text_sm())
            .child(
                ui::field()
                    .label(move || tr("Description", "群公告"))
                    .value(move || description.get())
                    .max_length(512)
                    .multiline(true)
                    .min_h_28()
                    .on_change(move |value: &str| {
                        description.set(value.into());
                        keep_draft();
                    }),
            )
            .child(
                ui::primary()
                    .disabled(move || {
                        state.busy.get()
                            || !state.can_manage()
                            || name.with(|value| value.trim().is_empty())
                            || (initial_name
                                .with(|initial| name.with(|value| value.trim() == initial.trim()))
                                && initial_description
                                    .with(|initial| description.with(|value| value == initial)))
                    })
                    .on_press(move || {
                        if state.busy.get_untracked() || !state.can_manage() {
                            return;
                        }
                        let client = state.client();
                        let id = state.id();
                        let name = name.get_untracked().trim().to_owned();
                        let description = description.get_untracked();
                        let name =
                            initial_name.with(|initial| (initial.trim() != name).then_some(name));
                        let description = initial_description
                            .with(|initial| (*initial != description).then_some(description));
                        if name.is_none() && description.is_none() {
                            return;
                        }
                        state.submit(
                            async move {
                                client
                                    .conversation()
                                    .edit_conversation(id, name, description)
                                    .await
                            },
                            move || {
                                state.edit_draft.replace(None);
                                state.page.set(Page::Info);
                            },
                        );
                    })
                    .child(text(move || tr("Save", "保存"))),
            ),
    )
}

const EXPIRY_UNITS: [(i64, i64, &str, &str); 5] = [
    (1, 59, "Seconds", "秒"),
    (60, 59, "Minutes", "分钟"),
    (3600, 23, "Hours", "小时"),
    (86400, 6, "Days", "天"),
    (604800, 4, "Weeks", "周"),
];

pub(super) fn expiry(state: State) -> impl IntoElement {
    let i18n = i18n::current();
    let value = move || {
        state
            .detail
            .with(|detail| detail.as_ref().map_or(0, |detail| detail.expire_in))
    };
    let custom = signal(String::new());
    let unit = signal(0_usize);
    let custom_seconds = computed(move || {
        let (factor, maximum, _, _) = EXPIRY_UNITS[unit.get()];
        custom
            .get()
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|amount| (1..=maximum).contains(amount))
            .and_then(|amount| amount.checked_mul(factor))
            .filter(|seconds| (1..=2_419_200).contains(seconds))
    });
    let invalid = signal(false);
    let disabled = move || state.busy.get() || (state.group() && !state.can_manage());
    let apply = move |seconds: i64| {
        if disabled() || value() == seconds {
            return;
        }
        let client = state.client();
        let id = state.id();
        state.submit(
            async move {
                client
                    .conversation()
                    .set_disappearing_messages(id, seconds)
                    .await
            },
            move || state.page.set(Page::Info),
        );
    };
    let unit_label = move || {
        let (_, _, en, zh) = EXPIRY_UNITS[unit.get()];
        tr(en, zh)
    };
    let quantity = view()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(text(move || tr("Quantity", "数量")).text_sm())
        .child(
            ui::field()
                .label(move || tr("Quantity", "数量"))
                .max_length(2)
                .disabled(disabled)
                .value(move || custom.get())
                .on_change(move |text: &str| {
                    custom.set(text.into());
                    invalid.set(false);
                }),
        );
    let time_unit = view()
        .flex_1()
        .min_w_0()
        .gap_1()
        .child(text(move || tr("Time unit", "时间单位")).text_sm())
        .child(
            parts::option_select()
                .items(move || {
                    EXPIRY_UNITS
                        .into_iter()
                        .enumerate()
                        .map(|(index, (_, _, en, zh))| {
                            SelectItem::new(index.to_string(), tr(en, zh))
                        })
                        .collect::<Vec<_>>()
                })
                .value(move || Some(SharedString::from(unit.get().to_string())))
                .disabled(disabled)
                .accessibility_label(move || Some(tr("Time unit", "时间单位").into()))
                .trigger(parts::select_trigger(unit_label))
                .on_change(move |next: &Option<SharedString>| {
                    if let Some(index) = next
                        .as_ref()
                        .and_then(|next| next.parse::<usize>().ok())
                        .filter(|index| *index < EXPIRY_UNITS.len())
                    {
                        unit.set(index);
                        invalid.set(false);
                    }
                }),
        );
    scroll().flex_1().min_h_0().child(
        view()
            .p_4()
            .gap_3()
            .child(
                text(move || {
                    tr(
                        "New messages disappear after the selected time.",
                        "新消息将在所选时间后自动消失。",
                    )
                })
                .text_sm()
                .text_color(move || theme().muted_foreground),
            )
            .children(
                [
                    (0, "Off", "关闭"),
                    (30, "30 seconds", "30 秒"),
                    (600, "10 minutes", "10 分钟"),
                    (7200, "2 hours", "2 小时"),
                    (86400, "1 day", "1 天"),
                    (604800, "1 week", "1 周"),
                ]
                .into_iter()
                .map(move |(seconds, en, zh)| {
                    ui::command()
                        .justify_start()
                        .selected(move || value() == seconds)
                        .disabled(disabled)
                        .on_press(move || apply(seconds))
                        .child(text(move || tr(en, zh)).flex_1())
                        .child(when(
                            move || value() == seconds,
                            || ui::icon(ui::Icon::Check),
                        ))
                }),
            )
            .child(section(move || tr("Custom time", "自定义时间")))
            .child(
                view()
                    .flex_row()
                    .items_end()
                    .gap_3()
                    .child(quantity)
                    .child(time_unit),
            )
            .child(
                text(move || {
                    i18n.format(
                        "Enter a whole number from 1 to {maximum}.",
                        "请输入 1 到 {maximum} 之间的整数。",
                        &[("maximum", &EXPIRY_UNITS[unit.get()].1.to_string())],
                    )
                })
                .text_sm()
                .text_color(move || {
                    if invalid.get() {
                        theme().destructive
                    } else {
                        theme().muted_foreground
                    }
                }),
            )
            .child(
                ui::primary()
                    .disabled(move || disabled() || custom_seconds.get().is_none())
                    .on_press(move || match custom_seconds.get_untracked() {
                        Some(seconds) => apply(seconds),
                        None => invalid.set(true),
                    })
                    .child(text(move || tr("Set", "设置"))),
            ),
    )
}

pub(super) fn invite(state: State) -> impl IntoElement {
    let qr = computed(move || crate::qr::source(&detail_url(state), 256));
    scroll().flex_1().min_h_0().child(
        view()
            .p_4()
            .gap_4()
            .items_center()
            .child(
                text(move || {
                    tr(
                        "Anyone with this link can join the group.",
                        "获得此链接的任何人都可以加入群组。",
                    )
                })
                .text_sm()
                .text_center(),
            )
            .child(dynamic(move || match qr.get() {
                Some(source) => image(source)
                    .size_40()
                    .label("Group invite QR code")
                    .into_element(),
                None => text(move || tr("Invitation is unavailable.", "邀请链接暂不可用。"))
                    .text_sm()
                    .into_element(),
            }))
            .child(text(move || detail_url(state)).text_sm().select_text())
            .child(
                ui::action(
                    move || tr("Copy link", "复制链接"),
                    move || set_clipboard(untracked(|| detail_url(state))),
                )
                .disabled(move || detail_url(state).is_empty()),
            )
            .child(
                ui::action(
                    move || tr("Reset link…", "重置链接…"),
                    move || {
                        let client = state.client();
                        let id = state.id();
                        state.confirm_then(
                            Prompt::new(
                                tr("Reset invitation link?", "重置邀请链接？"),
                                tr("The previous link will stop working.", "此前的邀请链接将失效。"),
                                tr("Reset", "重置"),
                            ),
                            move || async move {
                                client.conversation().rotate_group_invite(id).await
                            },
                        );
                    },
                )
                .disabled(move || state.busy.get() || !state.can_manage()),
            ),
    )
}
