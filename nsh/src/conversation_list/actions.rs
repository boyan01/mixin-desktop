use std::{future::Future, sync::Arc};

use mixin_desktop_api::{AccountClient, ClientResult, ConversationListData};
use nsh_base::{alert_dialog, menu_item, theme};
use nsh_core::prelude::*;

use crate::{
    i18n::{self, I18n, Key},
    runtime, ui,
};

#[derive(Clone, Copy)]
pub(crate) struct State {
    owner: Owner,
    account: Stored<Arc<AccountClient>>,
    selected: Signal<Option<ConversationListData>>,
    circles: crate::sidebar::Circles,
    muting: Signal<Option<ConversationListData>>,
    busy: Signal<bool>,
    failed: Signal<bool>,
}

impl State {
    pub fn new(
        account: Arc<AccountClient>,
        selected: Signal<Option<ConversationListData>>,
        circles: crate::sidebar::Circles,
    ) -> Self {
        Self {
            owner: Owner::current(),
            account: stored(account),
            selected,
            circles,
            muting: signal_with(None, SignalEquals::Never),
            busy: signal(false),
            failed: signal(false),
        }
    }

    fn execute(self, command: impl Future<Output = ClientResult<()>> + Send + 'static) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        let command = runtime::commit(command);
        self.owner.run(|| {
            spawn(async move {
                self.failed.set(command.await.is_err());
                self.busy.set(false);
            })
        });
    }
}

fn tr(i18n: I18n, en: &'static str, zh: &'static str) -> &'static str {
    i18n.tr(en, zh)
}

fn item(label: &'static str, action: impl Fn() + 'static) -> impl IntoElement {
    menu_item()
        .px_3()
        .py_2()
        .rounded_md()
        .hover(|s| s.bg(move || theme().accent))
        .focus(|s| s.bg(move || theme().accent))
        .on_select(action)
        .child(text(label).text_sm())
}

impl State {
    pub(crate) fn toggle_pin(self, row: ConversationListData) {
        let account = self.account.with(|account| account.clone());
        self.execute(async move {
            let access = account.conversation();
            let current = access
                .conversation_items_by_ids(vec![row.conversation_id.clone()])
                .await?
                .into_iter()
                .next()
                .ok_or(mixin_desktop_api::ClientError::NotFound)?;
            access
                .set_pinned(current.conversation_id, !current.is_pinned)
                .await
        });
    }

    pub(crate) fn toggle_mute(self, row: ConversationListData) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.failed.set(false);
        let account = self.account.with(|account| account.clone());
        self.owner.run(|| {
            spawn(async move {
                let client = account.clone();
                let result = runtime::run(async move {
                    client
                        .conversation()
                        .conversation_items_by_ids(vec![row.conversation_id])
                        .await?
                        .into_iter()
                        .next()
                        .ok_or(mixin_desktop_api::ClientError::NotFound)
                })
                .await;
                match result {
                    Ok(row) if row.is_muted => {
                        self.failed.set(
                            runtime::commit(async move {
                                account
                                    .conversation()
                                    .set_muted(row.conversation_id, row.owner_id, row.category, 0)
                                    .await
                            })
                            .await
                            .is_err(),
                        );
                    }
                    Ok(row) => self.muting.set(Some(row)),
                    Err(_) => self.failed.set(true),
                }
                self.busy.set(false);
            })
        });
    }

    pub(crate) fn remove(self, row: ConversationListData) {
        if self.busy.get_untracked() {
            return;
        }
        let i18n = i18n::current();
        let account = self.account.with(|account| account.clone());
        self.owner.run(|| {
            spawn(async move {
                let title =
                    i18n.format("Delete “{name}”?", "删除“{name}”？", &[("name", &row.name)]);
                let answer = alert_dialog()
                    .title(title)
                    .description(tr(
                        i18n,
                        "This removes the chat and its message history from this device.",
                        "此操作将从此设备删除会话及其聊天记录。",
                    ))
                    .cancel("cancel", i18n.text(Key::Cancel))
                    .destructive("delete", tr(i18n, "Delete", "删除"))
                    .show()
                    .await;
                if !answer.is_ok_and(|answer| answer.action.as_deref() == Some("delete"))
                    || self.busy.get_untracked()
                {
                    return;
                }
                self.busy.set(true);
                let id = row.conversation_id;
                let deleted_id = id.clone();
                let result =
                    runtime::commit(
                        async move { account.conversation().delete_conversation(id).await },
                    )
                    .await;
                self.failed.set(result.is_err());
                if result.is_ok()
                    && self.selected.with(|row| {
                        row.as_ref()
                            .is_some_and(|row| row.conversation_id == deleted_id)
                    })
                {
                    self.selected.set(None);
                }
                self.busy.set(false);
            })
        });
    }
}

pub(crate) fn wrap(
    state: State,
    row: impl Fn() -> Option<ConversationListData> + Copy + 'static,
    content: impl IntoElement + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::context_menu().trigger(content).child(dynamic(move || {
        let Some(row) = untracked(row) else {
            return view().hidden().into_element();
        };
        let pinned = row.is_pinned;
        let muted = row.is_muted;
        let row = stored(row);
        view()
            .child(item(
                tr(
                    i18n,
                    if pinned { "Unpin" } else { "Pin" },
                    if pinned { "取消置顶" } else { "置顶" },
                ),
                move || state.toggle_pin(row.with(|row| row.clone())),
            ))
            .child(item(
                tr(
                    i18n,
                    if muted { "Unmute" } else { "Mute…" },
                    if muted { "取消静音" } else { "静音…" },
                ),
                move || state.toggle_mute(row.with(|row| row.clone())),
            ))
            .child(crate::sidebar::circle_membership_item(
                state.circles,
                move || Some(row.with(|row| row.clone())),
            ))
            .child(item(
                tr(i18n, "Delete chat…", "删除会话…"),
                move || state.remove(row.with(|row| row.clone())),
            ))
            .into_element()
    }))
}

pub(crate) fn surface(state: State) -> impl IntoElement {
    let i18n = i18n::current();
    let presence = crate::motion::Presence::modal(move || state.muting.with(Option::is_some));
    view()
        .child(when(state.failed, move || {
            view()
                .px_4()
                .py_2()
                .flex_row()
                .gap_2()
                .child(
                    text(move || i18n.text(Key::CommandFailed))
                        .text_sm()
                        .text_color(move || theme().destructive)
                        .flex_1(),
                )
                .child(ui::action(
                    move || tr(i18n, "Dismiss", "关闭"),
                    move || state.failed.set(false),
                ))
        }))
        .child(when(
            move || presence.mounted(),
            move || {
                let duration = signal(None::<i64>);
                ui::modal(
                    presence,
                    move || {
                        if !state.busy.get_untracked() {
                            state.muting.set(None);
                        }
                    },
                    view()
                        .w_80()
                        .max_w(relative(1.0))
                        .p_6()
                        .gap_4()
                        .rounded_xl()
                        .bg(move || theme().popover)
                        .child(
                            text(move || tr(i18n, "Mute notifications", "通知静音"))
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD),
                        )
                        .children(
                            [
                                (3600, "1 hour", "1 小时"),
                                (28800, "8 hours", "8 小时"),
                                (604800, "1 week", "1 周"),
                                (31536000, "1 year", "1 年"),
                            ]
                            .into_iter()
                            .map(move |(seconds, en, zh)| {
                                ui::command()
                                    .justify_start()
                                    .selected(move || duration.get() == Some(seconds))
                                    .selected_style(|s| s.bg(move || theme().accent))
                                    .disabled(state.busy)
                                    .on_press(move || duration.set(Some(seconds)))
                                    .child(text(move || tr(i18n, en, zh)))
                            }),
                        )
                        .child(
                            view()
                                .flex_row()
                                .justify_end()
                                .gap_2()
                                .child(
                                    ui::action(
                                        move || i18n.text(Key::Cancel),
                                        move || state.muting.set(None),
                                    )
                                    .disabled(state.busy),
                                )
                                .child(
                                    ui::primary()
                                        .disabled(move || {
                                            duration.get().is_none() || state.busy.get()
                                        })
                                        .child(text(move || tr(i18n, "Mute", "静音")))
                                        .on_press(move || {
                                            let (Some(row), Some(seconds)) = (
                                                state.muting.get_untracked(),
                                                duration.get_untracked(),
                                            ) else {
                                                return;
                                            };
                                            let account =
                                                state.account.with(|account| account.clone());
                                            state.busy.set(true);
                                            spawn(async move {
                                                let result = runtime::commit(async move {
                                                    account
                                                        .conversation()
                                                        .set_muted(
                                                            row.conversation_id,
                                                            row.owner_id,
                                                            row.category,
                                                            seconds,
                                                        )
                                                        .await
                                                })
                                                .await;
                                                state.busy.set(false);
                                                state.failed.set(result.is_err());
                                                if result.is_ok() {
                                                    state.muting.set(None);
                                                }
                                            });
                                        }),
                                ),
                        ),
                )
            },
        ))
}
