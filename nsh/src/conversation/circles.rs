use mixin_desktop_api::CircleItem;

use super::*;

pub(super) fn screen(state: State) -> impl IntoElement {
    let circles = signal(Vec::<CircleItem>::new());
    let loaded = signal(false);
    let client = state.client();
    spawn(async move {
        let mut changes = Box::pin(crate::runtime::observe(client.circle_changes()));
        while let Some(rows) = changes.next().await {
            batch(|| {
                circles.set(rows);
                loaded.set(true);
            });
        }
    });
    let name = signal(String::new());
    let created = signal(None::<String>);
    let busy = signal(false);
    let failed = signal(false);
    let create = move || {
        if busy.get_untracked() || state.busy.get_untracked() {
            return;
        }
        let Some(conversation) = state.row.get_untracked() else {
            return;
        };
        let name_value = name.get_untracked().trim().to_owned();
        if name_value.is_empty() {
            return;
        }
        busy.set(true);
        failed.set(false);
        let client = state.client();
        let existing = created.get_untracked();
        let task = crate::runtime::commit(async move {
            let access = client.conversation();
            let id = match existing {
                Some(id) => id,
                None => access.create_circle(name_value).await?.circle_id,
            };
            let added = access
                .edit_circle_conversation(
                    id.clone(),
                    conversation.conversation_id,
                    conversation.owner_id,
                    conversation.category == "GROUP",
                    true,
                )
                .await;
            Ok((id, added.is_ok()))
        });
        spawn(async move {
            let result = task.await;
            busy.set(false);
            match result {
                Ok((_, true)) => {
                    created.set(None);
                    name.set(String::new());
                    state.reload();
                }
                Ok((id, false)) => {
                    created.set(Some(id));
                    failed.set(true);
                }
                Err(_) => failed.set(true),
            }
        });
    };
    view()
        .flex_1()
        .min_h_0()
        .child(
            parts::page_body()
                .child(parts::note(move || {
                    tr("Choose circles for this chat.", "选择此聊天所属的圈子。")
                }))
                .child(
                    ui::field()
                        .label(move || tr("New circle name", "新圈子名称"))
                        .placeholder(move || tr("New circle name", "新圈子名称").to_owned())
                        .max_length(64)
                        .readonly(move || busy.get() || created.with(Option::is_some))
                        .value(move || name.get())
                        .on_change(move |text: &str| name.set(text.into())),
                )
                .child(
                    ui::action(
                        move || {
                            if created.with(Option::is_some) {
                                tr("Retry adding chat", "重试添加聊天")
                            } else {
                                tr("Create circle", "创建圈子")
                            }
                        },
                        create,
                    )
                    .disabled(move || {
                        busy.get() || state.busy.get() || name.with(|name| name.trim().is_empty())
                    }),
                )
                .child(when(busy, move || {
                    parts::status(move || {
                        if created.with_untracked(Option::is_some) {
                            tr("Adding chat…", "正在添加聊天…")
                        } else {
                            tr(
                                "Creating circle and adding chat…",
                                "正在创建圈子并添加聊天…",
                            )
                        }
                    })
                }))
                .child(when(failed, move || {
                    ui::error_text(move || {
                        if created.with(Option::is_some) {
                            tr(
                                "Circle created, but this chat couldn’t be added. Retry adding it.",
                                "圈子已创建，但未能添加此聊天。请重试添加。",
                            )
                        } else {
                            tr(
                                "Couldn’t create the circle. Try again.",
                                "无法创建圈子，请重试。",
                            )
                        }
                    })
                })),
        )
        .child(
            virtual_list(
                move || circles.with(Vec::len),
                move |at| {
                    let circle = move || circles.with(|rows| rows.get(at.index()).cloned());
                    let checked = move || {
                        circle().is_some_and(|circle| {
                            state.row.with(|row| {
                                row.as_ref()
                                    .is_some_and(|row| row.circle_ids.contains(&circle.circle_id))
                            })
                        })
                    };
                    parts::compact_row()
                        .selected(checked)
                        .disabled(move || state.busy.get() || busy.get())
                        .accessibility_label(move || {
                            circle().map(|circle| {
                                format!(
                                    "{}: {}",
                                    circle.name,
                                    if checked() {
                                        tr("Added", "已添加")
                                    } else {
                                        tr("Not added", "未添加")
                                    }
                                )
                                .into()
                            })
                        })
                        .on_press(move || {
                            if let (Some(circle), Some(value)) =
                                (circle(), state.row.get_untracked())
                            {
                                let add = !checked();
                                let client = state.client();
                                state.submit(
                                    async move {
                                        client
                                            .conversation()
                                            .edit_circle_conversation(
                                                circle.circle_id,
                                                value.conversation_id,
                                                value.owner_id,
                                                value.category == "GROUP",
                                                add,
                                            )
                                            .await
                                    },
                                    || {},
                                );
                            }
                        })
                        .child(
                            text(move || circle().map(|circle| circle.name).unwrap_or_default())
                                .text_role(ui::TextRole::Label)
                                .flex_1(),
                        )
                        .child(when(checked, || ui::icon(ui::Icon::Check)))
                },
            )
            .keys(move || {
                circles.with(|rows| {
                    rows.iter()
                        .map(|circle| circle.circle_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .estimated_item_size(parts::row_height())
            .flex_1()
            .min_h_0()
            .style_when(move || circles.with(Vec::is_empty), |style| style.hidden()),
        )
        .child(when(
            move || loaded.get() && circles.with(Vec::is_empty),
            || ui::placeholder(move || tr("No circles yet", "暂无圈子")),
        ))
}
