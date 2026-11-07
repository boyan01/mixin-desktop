use std::{collections::BTreeMap, sync::Arc};

use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, CircleConversationEdit, CircleItem, ConversationListData};
use nsh_core::prelude::*;

use crate::{
    conversation::parts,
    i18n::tr,
    runtime,
    ui::{self, Typography as _},
};

const PAGE_SIZE: i64 = 60;

type Changes = BTreeMap<String, (ConversationListData, bool)>;

// Only unsaved presentation choices live here. Persisted membership is always re-read.
fn remaining(desired: Changes, rows: Vec<ConversationListData>, circle: &str) -> Changes {
    let rows = rows
        .into_iter()
        .map(|row| (row.conversation_id.clone(), row))
        .collect::<BTreeMap<_, _>>();
    desired
        .into_iter()
        .filter_map(|(id, (_, add))| {
            let row = rows.get(&id)?;
            (row.circle_ids.iter().any(|current| current == circle) != add)
                .then(|| (id, (row.clone(), add)))
        })
        .collect()
}

pub(super) fn screen(
    account: Arc<AccountClient>,
    circle: CircleItem,
    presence: crate::motion::Presence,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let account = stored(account);
    let circle_id = stored(circle.circle_id);
    let title = stored(circle.name);
    let rows = signal_with(Vec::<ConversationListData>::new(), SignalEquals::Never);
    let changes = signal_with(Changes::new(), SignalEquals::Never);
    let query = signal(String::new());
    let composing = signal(false);
    let page = signal(0_i64);
    let more = signal(false);
    let loading = signal(true);
    let busy = signal(false);
    let failed = signal(false);
    let load_failed = signal(false);
    let reconcile = signal(false);
    let refresh = signal(0_u64);
    let observer = account.with(|account| account.clone());
    spawn(async move {
        let mut stream = Box::pin(runtime::throttled(
            observer.conversation_changes(),
            std::time::Duration::from_millis(333),
        ));
        while stream.next().await.is_some() {
            refresh.update(|refresh| *refresh += 1);
        }
    });
    let seen_refresh = stored(0_u64);
    effect(move || {
        if composing.get() {
            return;
        }
        let keyword = query.get();
        let page = page.get();
        let revision = refresh.get();
        // A live update replaces the rows quietly; only a new query shows loading.
        let background = seen_refresh.replace(revision) != revision
            && !load_failed.get_untracked()
            && !loading.get_untracked();
        let client = account.with(|account| account.clone());
        if !background {
            loading.set(true);
            load_failed.set(false);
        }
        // Rerunning this effect cancels the previous task and its business call.
        spawn(async move {
            if !background && !keyword.trim().is_empty() {
                timer(std::time::Duration::from_millis(150)).await;
            }
            let result = runtime::run(async move {
                client
                    .conversation()
                    .conversations(
                        "chats".into(),
                        None,
                        keyword,
                        false,
                        PAGE_SIZE + 1,
                        page * PAGE_SIZE,
                    )
                    .await
            })
            .await;
            match result {
                Ok(mut values) => {
                    more.set(values.len() > PAGE_SIZE as usize);
                    values.truncate(PAGE_SIZE as usize);
                    rows.set(values);
                }
                Err(_) => load_failed.set(true),
            }
            loading.set(false);
        });
    });
    let reload_choices = move || {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        let desired = changes.get_untracked();
        let ids = desired.keys().cloned().collect();
        let client = account.with(|account| account.clone());
        let circle = circle_id.with(|id| id.clone());
        spawn(async move {
            match runtime::run(
                async move { client.conversation().conversation_items_by_ids(ids).await },
            )
            .await
            {
                Ok(values) => {
                    changes.set(remaining(desired, values, &circle));
                    reconcile.set(false);
                    failed.set(false);
                    refresh.update(|refresh| *refresh += 1);
                }
                Err(_) => failed.set(true),
            }
            busy.set(false);
        });
    };
    let save = move || {
        if !presence.interactive()
            || busy.get_untracked()
            || reconcile.get_untracked()
            || changes.with_untracked(BTreeMap::is_empty)
        {
            return;
        }
        busy.set(true);
        failed.set(false);
        let desired = changes.get_untracked();
        let persisted = desired.clone();
        let client = account.with(|account| account.clone());
        let circle = circle_id.with(|id| id.clone());
        let task = runtime::commit(async move {
            let access = client.conversation();
            let ids = persisted.keys().cloned().collect();
            let current = access.conversation_items_by_ids(ids).await?;
            let pending = remaining(persisted, current, &circle);
            let edits = pending
                .values()
                .map(|(row, add)| {
                    let mut edit = CircleConversationEdit::default();
                    edit.conversation_id = row.conversation_id.clone();
                    edit.owner_id = row.owner_id.clone();
                    edit.is_group = row.category == "GROUP";
                    edit.add = *add;
                    edit
                })
                .collect::<Vec<_>>();
            let saved = async {
                for batch in edits.chunks(200) {
                    access
                        .edit_circle_conversations(circle.clone(), batch.to_vec())
                        .await?;
                }
                Ok::<(), mixin_desktop_api::ClientError>(())
            }
            .await
            .is_ok();
            // A failed batch can already have persisted a prefix. Never retry that prefix.
            let current = access
                .conversation_items_by_ids(desired.keys().cloned().collect())
                .await?;
            Ok((saved, current))
        });
        spawn(async move {
            match task.await {
                Ok((saved, current)) => {
                    let desired = changes.get_untracked();
                    let circle = circle_id.with(|id| id.clone());
                    let pending = remaining(desired, current, &circle);
                    let complete = saved && pending.is_empty();
                    changes.set(pending);
                    failed.set(!complete);
                    reconcile.set(false);
                    busy.set(false);
                    refresh.update(|refresh| *refresh += 1);
                    if complete {
                        close();
                    }
                }
                Err(_) => {
                    failed.set(true);
                    reconcile.set(true);
                    busy.set(false);
                }
            }
        });
    };
    let card = parts::tall_dialog_card()
        .child(ui::dialog_title(move || {
            tr("Edit conversations", "编辑聊天")
        }))
        .child(text(move || title.with(|title| title.clone())).text_role(ui::TextRole::Body))
        .child(
            ui::search_pill().flex_none().child(
                ui::search_input()
                    .label(move || tr("Search chats", "搜索聊天"))
                    .placeholder(move || tr("Search chats", "搜索聊天").to_owned())
                    .value(move || query.get())
                    .on_change(move |text: &str| {
                        batch(|| {
                            query.set(text.into());
                            page.set(0);
                        })
                    })
                    .on_composition_change(move |event: &Composition| {
                        composing.set(event.composing)
                    }),
            ),
        )
        .child(when(
            move || loading.get() && rows.with(Vec::is_empty),
            || parts::status(move || tr("Loading…", "加载中…")),
        ))
        .child(when(load_failed, move || {
            view()
                .gap_2()
                .child(ui::error_text(move || {
                    tr("Couldn’t load chats.", "无法加载聊天。")
                }))
                .child(ui::action(
                    move || tr("Reload", "重新加载"),
                    move || refresh.update(|refresh| *refresh += 1),
                ))
        }))
        .child(when(failed, move || {
            view()
                .gap_2()
                .child(ui::error_text(move || {
                    if reconcile.get() {
                        tr(
                            "Couldn’t refresh saved changes. Reload before saving again.",
                            "无法刷新已保存的更改，请先重新加载。",
                        )
                    } else {
                        tr(
                            "Some changes could not be saved. Check the remaining selections.",
                            "部分更改未能保存，请检查剩余选择。",
                        )
                    }
                }))
                .child(ui::action(move || tr("Reload", "重新加载"), reload_choices).disabled(busy))
        }))
        .child(
            virtual_list(
                move || rows.with(Vec::len),
                move |at| {
                    let row = move || rows.with(|rows| rows.get(at.index()).cloned());
                    let selected = move || {
                        row().is_some_and(|row| {
                            changes
                                .with(|changes| {
                                    changes.get(&row.conversation_id).map(|(_, add)| *add)
                                })
                                .unwrap_or_else(|| circle_id.with(|id| row.circle_ids.contains(id)))
                        })
                    };
                    parts::compact_row()
                        .selected(selected)
                        .disabled(move || busy.get() || reconcile.get())
                        .child(ui::conversation_avatar(row, ui::AvatarSize::Medium))
                        .child(
                            text(move || row().map(|row| row.name).unwrap_or_default())
                                .text_role(ui::TextRole::Label)
                                .flex_1()
                                .number_of_lines(1),
                        )
                        .child(when(selected, || ui::icon(ui::Icon::Check)))
                        .on_press(move || {
                            if let Some(row) = row() {
                                let next = !selected();
                                let persisted = circle_id.with(|id| row.circle_ids.contains(id));
                                changes.update(|changes| {
                                    if next == persisted {
                                        changes.remove(&row.conversation_id);
                                    } else {
                                        changes.insert(row.conversation_id.clone(), (row, next));
                                    }
                                });
                            }
                        })
                },
            )
            .keys(move || {
                rows.with(|rows| {
                    rows.iter()
                        .map(|row| row.conversation_id.clone())
                        .collect::<Vec<_>>()
                })
            })
            .estimated_item_size(parts::row_height())
            .flex_1()
            .min_h_0(),
        )
        .child(parts::pager(
            page,
            move || more.get(),
            move || busy.get() || loading.get(),
        ))
        .child(
            parts::dialog_actions()
                .child(ui::action(move || tr("Cancel", "取消"), close).disabled(busy))
                .child(
                    ui::primary()
                        .disabled(move || {
                            busy.get() || reconcile.get() || changes.with(BTreeMap::is_empty)
                        })
                        .on_press(save)
                        .child(text(move || {
                            format!("{} ({})", tr("Save", "保存"), changes.with(BTreeMap::len))
                        })),
                ),
        );
    ui::dialog(
        presence,
        move || {
            if !busy.get_untracked() {
                close();
            }
        },
        card,
    )
}
