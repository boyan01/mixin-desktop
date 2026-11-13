use nsh_base::SelectItem;
use unicode_segmentation::UnicodeSegmentation as _;

use super::*;

fn exited_group(state: State) -> bool {
    state.group()
        && (state
            .row
            .with(|row| row.as_ref().is_some_and(|row| row.status == 3))
            || state.members.with(|members| {
                !members
                    .iter()
                    .any(|member| member.user_id == state.own_id())
            }))
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

/// The `size_7` of a shared app's icon.
const APP_ICON: f32 = 28.0;

pub(super) fn screen(state: State) -> impl IntoElement {
    let avatar = crate::avatar::Picker::new();
    let links = use_context::<crate::links::LinkNavigation>().expect("Home provides link routing");
    scroll().flex_1().min_h_0().child(
        parts::grouped_page()
            .gap(ui::space_lg())
            .child(identity(state))
            .child(description(state))
            .child(quick_actions(state))
            .child(when(
                move || !exited_group(state),
                move || about_rows(state, links),
            ))
            .child(
                parts::group()
                    .child(when(
                        move || !exited_group(state),
                        move || expiry_row(state),
                    ))
                    .child(
                        icon_row(
                            ui::Icon::Circle,
                            move || tr("Circles", "圈子"),
                            move || state.page.set(Page::Circles),
                        )
                        .child(ui::chevron()),
                    ),
            )
            .child(when(
                move || state.can_manage() || !state.group() && state.owner_id() != state.own_id(),
                move || edit_rows(state, avatar),
            ))
            .child(danger_rows(state))
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
                    .text_role(ui::TextRole::Caption)
                    .text_center()
                    .text_color(ui::fg_secondary)
                },
            ))
            .child(avatar.dialog(true, move |data| {
                if state.busy.get_untracked() || !state.can_manage() {
                    return;
                }
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
            })),
    )
}

/// What the chat is: picture, name and one secondary line, as iOS and Telegram
/// head an info page.
fn identity(state: State) -> impl IntoElement {
    view()
        .items_center()
        .px(ui::space_lg())
        .pb(ui::space_xs())
        .gap(ui::space_xs())
        .child(ui::conversation_avatar(
            move || state.row.get(),
            ui::AvatarSize::Profile,
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_center()
                .gap(ui::space_xs())
                .min_w_0()
                .mt(ui::space_xs())
                .child(
                    text(move || row_text(state, |row| row.name.clone()))
                        .text_role(ui::TextRole::Headline)
                        .text_center()
                        .min_w_0()
                        .number_of_lines(2),
                )
                .child(user::identity_badge(state.profile)),
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
            .text_role(ui::TextRole::Caption)
            .text_color(ui::fg_secondary)
            .select_text(),
        )
        .child(when(
            move || !state.group(),
            move || user::sidebar_add(state),
        ))
}

/// The commands people reach for most, each an icon over a short label.
/// The chat's own commands: one group of rows, as iOS and Telegram list them.
fn quick_actions(state: State) -> impl IntoElement {
    let navigation = use_context::<crate::navigation::ChatNavigation>()
        .expect("The signed-in session provides navigation");
    parts::group()
        .child(icon_row(
            ui::Icon::Search,
            move || tr("Search messages", "搜索聊天记录"),
            move || navigation.open_search_from_info(),
        ))
        .child(
            icon_row(
                ui::Icon::Image,
                move || tr("Shared media", "共享媒体"),
                move || state.page.set(Page::Shared),
            )
            .child(ui::chevron()),
        )
        .child(dynamic(move || {
            let muted = muted(state);
            untracked(|| {
                icon_row(
                    if muted {
                        ui::Icon::VolumeOff
                    } else {
                        ui::Icon::Bell
                    },
                    move || {
                        if muted {
                            tr("Unmute", "取消静音")
                        } else {
                            tr("Mute", "静音")
                        }
                    },
                    move || {
                        if muted {
                            set_mute(state, 0);
                        } else {
                            choose_mute(state);
                        }
                    },
                )
                // An exited group keeps the command in place, dimmed.
                .disabled(move || state.busy.get() || exited_group(state))
                .child(
                    text(move || mute_until(state))
                        .number_of_lines(1)
                        .text_role(ui::TextRole::Caption)
                        .text_color(ui::fg_secondary),
                )
            })
        }))
}

/// The group description or the person's biography, clamped until expanded.
fn description(state: State) -> impl IntoElement {
    let description = move || {
        if state.group() {
            state.detail.with(|detail| {
                detail
                    .as_ref()
                    .map(|detail| detail.announcement.clone())
                    .unwrap_or_default()
            })
        } else {
            state.profile.with(|profile| {
                profile
                    .as_ref()
                    .map(|profile| profile.biography.clone())
                    .unwrap_or_default()
            })
        }
    };
    let expanded = signal(false);
    let preview = computed(move || {
        description()
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
            .graphemes(true)
            .take(60)
            .collect::<String>()
    });
    let truncated = move || preview.get().len() < description().trim_end().len();
    when(
        move || !description().is_empty(),
        move || {
            parts::group()
                .px(ui::space_lg())
                .py(ui::space_md())
                .gap(ui::space_xs())
                .child(
                    text(move || {
                        if expanded.get() {
                            description()
                        } else if truncated() {
                            format!("{}…", preview.get())
                        } else {
                            description()
                        }
                    })
                    .text_role(ui::TextRole::Body)
                    .select_text(),
                )
                .child(when(truncated, move || {
                    ui::link(
                        move || {
                            if expanded.get() {
                                tr("Show less", "收起")
                            } else {
                                tr("Show more", "展开")
                            }
                        },
                        move || expanded.set(!expanded.get()),
                    )
                    .self_start()
                }))
        },
    )
}

/// The people and things around the chat. Every row leads with an icon.
fn about_rows(state: State, links: crate::links::LinkNavigation) -> impl IntoElement {
    parts::group()
        .child(when(
            move || state.group(),
            move || {
                icon_row(
                    ui::Icon::Groups,
                    move || tr("Group participants", "群组成员"),
                    move || state.page.set(Page::Members),
                )
                .child(ui::chevron())
            },
        ))
        .child(when(
            move || !state.group(),
            move || {
                view()
                    .child(icon_row(
                        ui::Icon::Share,
                        move || tr("Share contact…", "分享联系人…"),
                        move || links.open(user::contact_uri(&state.owner_id()), None),
                    ))
                    .child(
                        icon_row(
                            ui::Icon::Groups,
                            move || tr("Groups in common", "共同群组"),
                            move || state.page.set(Page::Groups),
                        )
                        .child(ui::chevron()),
                    )
            },
        ))
        .child(when(
            move || state.shared_apps_preview.with(|apps| !apps.is_empty()),
            move || shared_apps_row(state),
        ))
        .child(when(
            move || state.creator.with(Option::is_some),
            move || {
                icon_row(
                    ui::Icon::Person,
                    move || tr("Created by", "开发者"),
                    move || {
                        if let Some(id) = state.creator.get_untracked() {
                            links.open(format!("mixin://users/{id}"), None);
                        }
                    },
                )
            },
        ))
}

/// Renames and the group's own picture and description. No row has an icon.
fn edit_rows(state: State, avatar: crate::avatar::Picker) -> impl IntoElement {
    parts::group()
        .child(row(
            move || tr("Edit name…", "编辑名称…"),
            move || state.page.set(Page::Edit),
        ))
        .child(when(
            move || state.group() && state.can_manage(),
            move || {
                view()
                    .child(
                        row(
                            move || tr("Change group photo…", "更换群头像…"),
                            move || {
                                if state.busy.get_untracked() || !state.can_manage() {
                                    return;
                                }
                                spawn(async move {
                                    state.failed.set(false);
                                    if avatar.pick().await.is_err() {
                                        state.failed.set(true);
                                    }
                                });
                            },
                        )
                        .disabled(move || state.busy.get() || avatar.busy()),
                    )
                    .child(row(
                        move || {
                            if state.detail.with(|detail| {
                                detail
                                    .as_ref()
                                    .is_none_or(|detail| detail.announcement.is_empty())
                            }) {
                                tr("Add group description…", "添加群公告…")
                            } else {
                                tr("Edit group description…", "编辑群公告…")
                            }
                        },
                        move || state.page.set(Page::Description),
                    ))
            },
        ))
}

fn shared_apps_row(state: State) -> impl IntoElement {
    icon_row(
        ui::Icon::Bot,
        move || tr("Shared apps", "共享应用"),
        move || state.page.set(Page::Apps),
    )
    .child(dynamic(move || {
        let apps = state.shared_apps_preview.get();
        // Content-derived: each icon overlaps the last by half its width.
        let width = APP_ICON + apps.len().saturating_sub(1) as f32 * APP_ICON / 2.0;
        view()
            .relative()
            .h_7()
            .w(px(width))
            .flex_shrink_0()
            .children(apps.into_iter().enumerate().rev().map(|(index, app)| {
                ui::avatar_view(
                    move || app.app_id.clone(),
                    move || app.name.clone().into(),
                    move || app.icon_url.clone(),
                    ui::AvatarSize::Medium,
                )
                .absolute()
                .top_0()
                .left(px(index as f32 * APP_ICON / 2.0))
                .size_7()
                .border_2()
                .border_color(ui::surface_list)
            }))
    }))
    .child(ui::chevron())
}

fn expiry_row(state: State) -> impl IntoElement {
    let label = move || tr("Disappearing messages", "阅后即焚");
    let value = move || {
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
        .text_role(ui::TextRole::Body)
        .text_color(ui::fg_secondary)
    };
    dynamic(move || {
        let readonly = state.group() && !state.can_manage();
        untracked(|| {
            if readonly {
                parts::group_static_row(Some(ui::Icon::Disappearing), label)
                    .child(value())
                    .into_element()
            } else {
                icon_row(ui::Icon::Disappearing, label, move || {
                    state.page.set(Page::Expiry)
                })
                .disabled(state.busy)
                .child(value())
                .child(ui::chevron())
                .into_element()
            }
        })
    })
}

/// When a muted chat speaks again, or nothing when it is not muted.
fn mute_until(state: State) -> String {
    if !muted(state) {
        return String::new();
    }
    let millis = state
        .detail
        .with(|detail| detail.as_ref().map_or(0, |detail| detail.mute_until_millis));
    chrono::DateTime::from_timestamp_millis(millis)
        .filter(|_| millis > 0)
        .map(|date| {
            i18n::current().format(
                "Muted until {arg0}",
                "静音至 {arg0}",
                &[(
                    "arg0",
                    &format!(
                        "{}, {}",
                        crate::presentation::date(i18n::current(), millis),
                        date.with_timezone(&chrono::Local).format("%H:%M")
                    ),
                )],
            )
        })
        .unwrap_or_default()
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

/// Destructive commands, last and apart from the rest, in the danger color.
fn danger_rows(state: State) -> impl IntoElement {
    let other = move || !state.group() && state.owner_id() != state.own_id();
    parts::group()
        .child(when(other, move || user::sidebar_relationship(state)))
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
            .text_color(ui::danger),
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
                .text_color(ui::danger)
            },
        ))
        .child(when(other, move || user::sidebar_report(state)))
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

pub(super) fn edit(state: State, description: bool) -> impl IntoElement {
    let initial = if !description && !state.group() {
        row_text(state, |row| row.name.clone())
    } else {
        state.detail.with_untracked(|detail| {
            detail
                .as_ref()
                .map(|detail| {
                    if description {
                        detail.announcement.clone()
                    } else {
                        detail.name.clone()
                    }
                })
                .unwrap_or_default()
        })
    };
    let initial = stored(initial);
    let draft = state
        .edit_draft
        .with(|(name, announcement)| {
            if description {
                announcement.clone()
            } else {
                name.clone()
            }
        })
        .unwrap_or_else(|| initial.get());
    let value = signal(draft);
    let keep = move |next: &str| {
        value.set(next.into());
        let (name, announcement) = state.edit_draft.get();
        state.edit_draft.replace(if description {
            (name, Some(next.into()))
        } else {
            (Some(next.into()), announcement)
        });
    };
    let disabled = move || {
        state.busy.get()
            || if state.group() {
                !state.can_manage()
            } else {
                state.profile.with(|profile| {
                    profile
                        .as_ref()
                        .is_none_or(|profile| profile.is_deactivated)
                })
            }
    };
    let label = move || {
        if description {
            tr("Description", "群公告")
        } else {
            tr("Name", "名称")
        }
    };
    let field = if description {
        ui::textarea()
            .label(label)
            .value(move || value.get())
            .max_length(512)
            .min_h_40()
            .readonly(disabled)
            .auto_focus(true)
            .on_change(keep)
            .into_element()
    } else {
        ui::field()
            .label(label)
            .value(move || value.get())
            .max_length(40)
            .readonly(disabled)
            .auto_focus(true)
            .on_change(keep)
            .into_element()
    };
    parts::page_body()
        .gap_4()
        .child(parts::labeled(label).child(field))
        .child(
            ui::primary()
                .self_end()
                .disabled(move || {
                    disabled()
                        || (!description && value.with(|v| v.trim().is_empty()))
                        || initial.with(|i| {
                            value.with(|v| {
                                if description {
                                    i == v
                                } else {
                                    i.trim() == v.trim()
                                }
                            })
                        })
                })
                .on_press(move || {
                    if disabled() {
                        return;
                    }
                    let client = state.client();
                    let id = state.id();
                    let user = state.owner_id();
                    let group = state.group();
                    let value = if description {
                        value.get_untracked()
                    } else {
                        value.get_untracked().trim().to_owned()
                    };
                    state.submit(
                        async move {
                            if group {
                                client
                                    .conversation()
                                    .edit_conversation(
                                        id,
                                        (!description).then(|| value.clone()),
                                        description.then_some(value),
                                    )
                                    .await
                            } else {
                                client.user().rename_contact(user, value).await
                            }
                        },
                        move || {
                            let (name, announcement) = state.edit_draft.get();
                            state.edit_draft.replace(if description {
                                (name, None)
                            } else {
                                (None, announcement)
                            });
                            state.page.set(Page::Info);
                        },
                    );
                })
                .child(text(move || tr("Save", "保存"))),
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
    let quantity = parts::labeled(move || tr("Quantity", "数量"))
        .flex_1()
        .min_w_0()
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
    let time_unit = parts::labeled(move || tr("Time unit", "时间单位"))
        .flex_1()
        .min_w_0()
        .child(
            ui::select()
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
                .trigger(ui::select_trigger(unit_label))
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
        parts::grouped_page()
            .child(parts::note(move || {
                tr(
                    "New messages disappear after the selected time.",
                    "新消息将在所选时间后自动消失。",
                )
            }))
            .child(
                parts::group().children(
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
                        parts::group_row(None, move || tr(en, zh))
                            .selected(move || value() == seconds)
                            .disabled(disabled)
                            .on_press(move || apply(seconds))
                            .child(when(
                                move || value() == seconds,
                                || ui::icon(ui::Icon::Check),
                            ))
                    }),
                ),
            )
            .child(
                // Its own section: the band, then its title and fields inside.
                parts::group().child(
                    view()
                        .p(ui::space_lg())
                        .gap(ui::space_md())
                        .child(ui::section_label(move || tr("Custom time", "自定义时间")))
                        .child(
                            view()
                                .flex_row()
                                .items_end()
                                .gap(ui::space_md())
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
                            .text_role(ui::TextRole::Body)
                            .text_color(move || {
                                if invalid.get() {
                                    ui::danger()
                                } else {
                                    ui::fg_secondary()
                                }
                            }),
                        )
                        .child(
                            ui::primary()
                                .self_end()
                                .disabled(move || disabled() || custom_seconds.get().is_none())
                                .on_press(move || match custom_seconds.get_untracked() {
                                    Some(seconds) => apply(seconds),
                                    None => invalid.set(true),
                                })
                                .child(text(move || tr("Set", "设置"))),
                        ),
                ),
            ),
    )
}
