use std::collections::BTreeMap;

use super::*;
use crate::{
    recipients::{self, Recipient},
    runtime,
};
use mixin_desktop_api::CircleConversationEdit;

pub(super) fn screen(
    account: Arc<AccountClient>,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let account = stored(account);
    let name = signal(String::new());
    let selecting = signal(false);
    let composing = signal(false);
    let selected = signal_with(BTreeMap::<String, Recipient>::new(), SignalEquals::Never);
    let failed = signal(false);
    let busy = signal(false);
    let created = signal(None::<String>);
    let presence =
        use_context::<crate::motion::Presence>().expect("Creation surface provides motion");
    let cancel = move || {
        if presence.interactive() && !busy.get_untracked() && !composing.get_untracked() {
            close();
        }
    };
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
                        Recipient::Chat(row) => row.conversation_id,
                        Recipient::User(user) => {
                            access.open_user_conversation(user.user_id).await?
                        }
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
    let picker = stored(Box::new(move || {
        recipients::picker(
            account.with(|account| account.clone()),
            move || busy.get() || created.with(Option::is_some),
            move |choice| selected.with(|chosen| chosen.contains_key(&choice.key())),
            cancel,
            move |choice| {
                selected.update(|chosen| {
                    let key = choice.key();
                    if chosen.remove(&key).is_none() {
                        chosen.insert(key, choice);
                    }
                })
            },
        )
        .into_element()
    }) as Box<dyn Fn() -> AnyElement>);
    modal(
        cancel,
        form(
            name, selecting, composing, selected, failed, busy, created, picker, cancel, create,
            true,
        ),
    )
}

#[allow(clippy::too_many_arguments)]
fn form(
    name: Signal<String>,
    selecting: Signal<bool>,
    composing: Signal<bool>,
    selected: Signal<BTreeMap<String, Recipient>>,
    failed: Signal<bool>,
    busy: Signal<bool>,
    created: Signal<Option<String>>,
    picker: Stored<Box<dyn Fn() -> AnyElement>>,
    cancel: impl Fn() + Copy + 'static,
    create: impl Fn() + Copy + 'static,
    focus: bool,
) -> View {
    panel()
        .child(ui::dialog_title(move || tr("New circle", "新建圈子")))
        .child(when(
            move || !selecting.get(),
            move || {
                ui::field()
                    .flex_none()
                    .label(move || tr("Circle name", "圈子名称"))
                    .placeholder(move || tr("Circle name", "圈子名称").to_owned())
                    .auto_focus(focus)
                    .max_length(64)
                    .value(move || name.get())
                    .on_key_press(move |key: &str| {
                        if key == "Escape" {
                            cancel();
                        }
                    })
                    .on_change(move |value: &str| name.set(value.into()))
                    .on_composition_change(move |event: &Composition| {
                        composing.set(event.composing)
                    })
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
                        .text_role(ui::TextRole::Body)
                        .number_of_lines(1),
                )
                .child(picker.with(|picker| picker()))
        }))
        .child(when(failed, move || {
            ui::error_text(move || {
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
                    .text_role(ui::TextRole::Caption)
                    .number_of_lines(1)
                    .text_color(ui::fg_secondary)
                })))
                .child(ui::action(move || tr("Cancel", "取消"), cancel).disabled(busy))
                .child(when(selecting, move || {
                    ui::action(move || tr("Back", "返回"), move || selecting.set(false))
                        .disabled(move || busy.get() || created.with(Option::is_some))
                }))
                .child(
                    ui::primary()
                        .disabled(move || busy.get() || name.with(|name| name.trim().is_empty()))
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
        )
}

#[cfg(debug_assertions)]
pub(crate) fn preview(
    users: Vec<mixin_desktop_api::UserProfileItem>,
    selecting: Signal<bool>,
    failed: Signal<bool>,
) -> impl IntoElement {
    let users = stored(users);
    let selected = signal_with(BTreeMap::<String, Recipient>::new(), SignalEquals::Never);
    let picker = stored(Box::new(move || {
        recipients::detached(
            users.get(),
            move |choice| selected.with(|s| s.contains_key(&choice.key())),
            move |choice| {
                selected.update(|s| {
                    let key = choice.key();
                    if s.remove(&key).is_none() {
                        s.insert(key, choice);
                    }
                });
            },
            true,
        )
        .into_element()
    }) as Box<dyn Fn() -> AnyElement>);
    let name = signal(String::new());
    effect(move || {
        if selecting.get() && name.with(|name| name.is_empty()) {
            name.set("Design review".into());
        }
    });
    form(
        name,
        selecting,
        signal(false),
        selected,
        failed,
        signal(false),
        signal(None),
        picker,
        || {},
        || {},
        false,
    )
}
