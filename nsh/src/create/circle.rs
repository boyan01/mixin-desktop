use std::collections::BTreeMap;

use super::*;
use crate::runtime;
use mixin_desktop_api::{CircleConversationEdit, UserProfileItem};

#[derive(Clone)]
enum Choice {
    Chat(Box<ConversationListData>),
    User(UserProfileItem),
}
impl Choice {
    fn key(&self) -> String {
        match self {
            Self::Chat(row) if row.category == "GROUP" => format!("chat:{}", row.conversation_id),
            Self::Chat(row) => format!("user:{}", row.owner_id),
            Self::User(row) => format!("user:{}", row.user_id),
        }
    }
    fn name(&self) -> String {
        match self {
            Self::Chat(row) => row.name.clone(),
            Self::User(row) => row.full_name.clone(),
        }
    }
    fn avatar(&self) -> String {
        match self {
            Self::Chat(row) => row.avatar_url.clone(),
            Self::User(row) => row.avatar_url.clone(),
        }
    }
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let account = stored(account);
    let name = signal(String::new());
    let selecting = signal(false);
    let contacts = signal(false);
    let query = signal(String::new());
    let composing = signal(false);
    let page = signal(0_i64);
    let rows = signal_with(Vec::<Choice>::new(), SignalEquals::Never);
    let selected = signal_with(BTreeMap::<String, Choice>::new(), SignalEquals::Never);
    let loading = signal(false);
    let more = signal(false);
    let failed = signal(false);
    let list_failed = signal(false);
    let busy = signal(false);
    let created = signal(None::<String>);
    let request = signal(0_u64);
    let revision = signal(0_u64);
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            close();
        }
    };
    let cancel_key = move |key: &str| {
        if key == "Escape" {
            cancel();
        }
    };
    const SIZE: i64 = 60;
    const ROW_HEIGHT: f32 = 64.0;
    const VISIBLE_ROWS: usize = 5;
    effect(move || {
        if !selecting.get() || composing.get() {
            return;
        }
        let contacts = contacts.get();
        let query = query.get();
        let page = page.get();
        let _revision = revision.get();
        let ticket = request.get_untracked().wrapping_add(1);
        request.set(ticket);
        let client = account.with(|account| account.clone());
        loading.set(true);
        list_failed.set(false);
        spawn(async move {
            if !query.trim().is_empty() {
                timer(std::time::Duration::from_millis(150)).await;
            }
            let result = runtime::run(async move {
                if contacts {
                    Ok(client
                        .user()
                        .selectable_users_page(query, page * SIZE, SIZE + 1)
                        .await?
                        .into_iter()
                        .map(Choice::User)
                        .collect::<Vec<_>>())
                } else {
                    Ok(client
                        .conversation()
                        .conversations("chats".into(), None, query, false, SIZE + 1, page * SIZE)
                        .await?
                        .into_iter()
                        .map(|row| Choice::Chat(Box::new(row)))
                        .collect())
                }
            })
            .await;
            if request.get_untracked() != ticket {
                return;
            }
            match result {
                Ok(mut values) => {
                    more.set(values.len() > SIZE as usize);
                    values.truncate(SIZE as usize);
                    rows.set(values);
                }
                Err(_) => list_failed.set(true),
            }
            loading.set(false);
        });
    });
    let create = move || {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = account.with(|account| account.clone());
        let name = name.get_untracked().trim().to_owned();
        let existing = created.get_untracked();
        let choices =
            selected.with_untracked(|choices| choices.values().cloned().collect::<Vec<_>>());
        let task = runtime::commit(async move {
            let access = client.conversation();
            let circle = match existing {
                Some(id) => id,
                None => access.create_circle(name).await?.circle_id,
            };
            let adding = async {
                let mut ids = Vec::new();
                for choice in choices {
                    ids.push(match choice {
                        Choice::Chat(row) => row.conversation_id,
                        Choice::User(user) => access.open_user_conversation(user.user_id).await?,
                    });
                }
                let expected = ids.len();
                let rows = access.conversation_items_by_ids(ids).await?;
                if rows.len() != expected {
                    return Err(mixin_desktop_api::ClientError::NotFound);
                }
                let edits = rows
                    .into_iter()
                    .filter(|row| !row.circle_ids.contains(&circle))
                    .map(|row| {
                        let mut edit = CircleConversationEdit::default();
                        edit.conversation_id = row.conversation_id;
                        edit.owner_id = row.owner_id;
                        edit.is_group = row.category == "GROUP";
                        edit.add = true;
                        edit
                    })
                    .collect::<Vec<_>>();
                for batch in edits.chunks(200) {
                    access
                        .edit_circle_conversations(circle.clone(), batch.to_vec())
                        .await?;
                }
                Ok::<(), mixin_desktop_api::ClientError>(())
            }
            .await;
            Ok((circle, adding.is_ok()))
        });
        spawn(async move {
            let result = task.await;
            busy.set(false);
            match result {
                Ok((id, true)) => {
                    created.set(Some(id));
                    close();
                }
                Ok((id, false)) => {
                    created.set(Some(id));
                    failed.set(true);
                }
                Err(_) => failed.set(true),
            }
        });
    };
    modal(cancel).child(
        panel()
            .h(auto())
            .min_h_0()
            .overflow_hidden()
            .child(
                text(move || tr("New circle", "新建圈子"))
                    .flex_none()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(when(
                move || !selecting.get(),
                move || {
                    view().flex_none().gap_4().child(
                        ui::field()
                            .label(move || tr("Circle name", "圈子名称"))
                            .placeholder(move || tr("Circle name", "圈子名称").to_owned())
                            .auto_focus(true)
                            .max_length(64)
                            .value(move || name.get())
                            .on_key_press(cancel_key)
                            .on_change(move |value: &str| name.set(value.into()))
                            .on_composition_change(move |event: &Composition| {
                                composing.set(event.composing)
                            }),
                    )
                },
            ))
            .child(when(selecting, move || {
                view()
                    .flex_shrink(1.0)
                    .min_h_0()
                    .gap_3()
                    .child(
                        text(move || name.get())
                            .flex_none()
                            .text_sm()
                            .number_of_lines(1),
                    )
                    .child(
                        view().flex_none().flex_row().gap_2().children(
                            [
                                (false, "Recent chats", "最近聊天"),
                                (true, "Contacts", "联系人"),
                            ]
                            .into_iter()
                            .map(move |(value, en, zh)| {
                                ui::command()
                                    .flex_1()
                                    .selected(move || contacts.get() == value)
                                    .selected_style(|style| style.bg(move || theme().accent))
                                    .on_press(move || {
                                        batch(|| {
                                            contacts.set(value);
                                            page.set(0);
                                            rows.set(Vec::new());
                                        })
                                    })
                                    .child(text(move || tr(en, zh)))
                            }),
                        ),
                    )
                    .child(
                        ui::field()
                            .label(move || tr("Search", "搜索"))
                            .placeholder(move || tr("Search", "搜索").to_owned())
                            .auto_focus(true)
                            .value(move || query.get())
                            .on_key_press(cancel_key)
                            .on_change(move |value: &str| {
                                batch(|| {
                                    query.set(value.into());
                                    page.set(0);
                                })
                            })
                            .on_composition_change(move |event: &Composition| {
                                composing.set(event.composing)
                            }),
                    )
                    .child(when(loading, || {
                        text(move || tr("Loading…", "加载中…")).text_sm()
                    }))
                    .child(when(
                        move || !loading.get() && !list_failed.get() && rows.with(Vec::is_empty),
                        || {
                            view().min_h_24().items_center().justify_center().child(
                                text(move || tr("No chats found", "没有找到聊天"))
                                    .text_sm()
                                    .text_color(move || theme().muted_foreground),
                            )
                        },
                    ))
                    .child(
                        virtual_list(
                            move || rows.with(Vec::len),
                            move |at| {
                                let choice =
                                    move || rows.with(|rows| rows.get(at.index()).cloned());
                                ui::command()
                                    .justify_start()
                                    .min_h_16()
                                    .gap_3()
                                    .disabled(move || busy.get() || created.with(Option::is_some))
                                    .selected(move || {
                                        choice().is_some_and(|choice| {
                                            selected.with(|selected| {
                                                selected.contains_key(&choice.key())
                                            })
                                        })
                                    })
                                    .selected_style(|style| style.bg(move || theme().accent))
                                    .on_press(move || {
                                        if let Some(choice) = choice() {
                                            selected.update(|selected| {
                                                if selected.remove(&choice.key()).is_none() {
                                                    selected.insert(choice.key(), choice);
                                                }
                                            });
                                        }
                                    })
                                    .child(ui::avatar_view(
                                        move || {
                                            choice()
                                                .map(|choice| choice.name().into())
                                                .unwrap_or_default()
                                        },
                                        move || {
                                            choice()
                                                .map(|choice| choice.avatar())
                                                .unwrap_or_default()
                                        },
                                    ))
                                    .child(
                                        text(move || {
                                            choice().map(|choice| choice.name()).unwrap_or_default()
                                        })
                                        .text_sm()
                                        .flex_1()
                                        .min_w_0()
                                        .number_of_lines(1),
                                    )
                                    .child(when(
                                        move || {
                                            choice().is_some_and(|choice| {
                                                selected.with(|selected| {
                                                    selected.contains_key(&choice.key())
                                                })
                                            })
                                        },
                                        || ui::icon(ui::Icon::Check),
                                    ))
                            },
                        )
                        .keys(move || {
                            rows.with(|rows| rows.iter().map(Choice::key).collect::<Vec<_>>())
                        })
                        .estimated_item_size(px(ROW_HEIGHT))
                        .h(move || {
                            px(rows.with(|rows| rows.len().min(VISIBLE_ROWS)) as f32 * ROW_HEIGHT)
                        })
                        .flex_shrink(1.0)
                        .min_h_0()
                        .style_when(move || rows.with(Vec::is_empty), |style| style.hidden()),
                    )
                    .child(when(
                        move || page.get() > 0 || more.get(),
                        move || {
                            view()
                                .flex_none()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    ui::action(
                                        move || tr("Previous page", "上一页"),
                                        move || page.update(|page| *page = page.saturating_sub(1)),
                                    )
                                    .disabled(move || loading.get() || page.get() == 0),
                                )
                                .child(text(move || (page.get() + 1).to_string()).text_sm())
                                .child(
                                    ui::action(
                                        move || tr("Next page", "下一页"),
                                        move || page.update(|page| *page += 1),
                                    )
                                    .disabled(move || loading.get() || !more.get()),
                                )
                        },
                    ))
            }))
            .child(when(
                move || selecting.get() && list_failed.get(),
                move || {
                    view()
                        .gap_2()
                        .child(
                            text(move || {
                                tr("Couldn’t load chats. Try again.", "聊天加载失败，请重试。")
                            })
                            .text_sm()
                            .text_color(move || theme().destructive),
                        )
                        .child(
                            ui::action(
                                move || tr("Reload list", "重新加载列表"),
                                move || revision.update(|revision| *revision += 1),
                            )
                            .disabled(busy),
                        )
                },
            ))
            .child(when(failed, move || {
                view().gap_2().child(
                    text(move || {
                        if created.with(Option::is_some) {
                            tr(
                                "The circle was created. Retry adding the remaining chats.",
                                "圈子已创建，请重试添加剩余聊天。",
                            )
                        } else {
                            tr(
                                "Couldn’t create the circle. Try again.",
                                "无法创建圈子，请重试。",
                            )
                        }
                    })
                    .text_sm()
                    .text_color(move || theme().destructive),
                )
            }))
            .child(
                view()
                    .flex_none()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(view().flex_1().min_w_0().child(when(selecting, move || {
                        text(move || {
                            i18n::current().format(
                                "{count} chats",
                                "{count} 个聊天",
                                &[("count", &selected.with(BTreeMap::len).to_string())],
                            )
                        })
                        .text_xs()
                        .number_of_lines(1)
                        .text_color(move || theme().muted_foreground)
                    })))
                    .child(ui::action(move || tr("Cancel", "取消"), cancel).disabled(busy))
                    .child(when(selecting, move || {
                        ui::action(move || tr("Back", "返回"), move || selecting.set(false))
                            .disabled(move || busy.get() || created.with(Option::is_some))
                    }))
                    .child(
                        ui::primary()
                            .disabled(move || {
                                busy.get() || name.with(|name| name.trim().is_empty())
                            })
                            .on_press(move || {
                                if selecting.get_untracked() {
                                    create();
                                } else {
                                    selecting.set(true);
                                }
                            })
                            .child(text(move || {
                                if created.with(Option::is_some) {
                                    tr("Retry", "重试")
                                } else if selecting.get() {
                                    tr("Create", "创建")
                                } else {
                                    tr("Next", "下一步")
                                }
                            })),
                    ),
            ),
    )
}
