use std::{future::Future, sync::Arc};

use mixin_desktop_api::{AccountClient, ClientResult, ConversationListData};
use nsh_base::alert_dialog;
use nsh_core::{element::desktop::MenuItem, prelude::*};

use crate::{
    conversation::parts,
    i18n::{self, Key},
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

impl State {
    pub(crate) fn toggle_pin(self, row: ConversationListData) {
        let account = self.account.with(|account| account.clone());
        self.execute(async move {
            let current = latest(&account, row.conversation_id).await?;
            account
                .conversation()
                .set_pinned(current.conversation_id, !current.is_pinned)
                .await
        });
    }

    /// Unmutes at once; muting first asks for how long.
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
                let result =
                    runtime::run(async move { latest(&client, row.conversation_id).await }).await;
                match result {
                    Ok(row) if row.is_muted => {
                        let result = runtime::commit(async move {
                            account
                                .conversation()
                                .set_muted(row.conversation_id, row.owner_id, row.category, 0)
                                .await
                        })
                        .await;
                        self.failed.set(result.is_err());
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
                    .description(i18n.tr(
                        "This removes the chat and its message history from this device.",
                        "此操作将从此设备删除会话及其聊天记录。",
                    ))
                    .cancel("cancel", i18n.text(Key::Cancel))
                    .destructive("delete", i18n.tr("Delete", "删除"))
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

async fn latest(
    account: &AccountClient,
    conversation_id: String,
) -> ClientResult<ConversationListData> {
    account
        .conversation()
        .conversation_items_by_ids(vec![conversation_id])
        .await?
        .into_iter()
        .next()
        .ok_or(mixin_desktop_api::ClientError::NotFound)
}

pub(crate) fn wrap(
    state: State,
    row: impl Fn() -> Option<ConversationListData> + Copy + 'static,
    content: impl IntoElement + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    ui::native_context_menu(content, move || {
        let Some(row) = row() else {
            return Vec::new();
        };
        let pin_title = if row.is_pinned {
            i18n.tr("Unpin", "取消置顶")
        } else {
            i18n.tr("Pin", "置顶")
        };
        let mute_title = if row.is_muted {
            i18n.tr("Unmute", "取消静音")
        } else {
            i18n.tr("Mute…", "静音…")
        };
        let pin = row.clone();
        let mute = row.clone();
        vec![
            MenuItem::new(pin_title).on_select(move || state.toggle_pin(pin.clone())),
            MenuItem::new(mute_title).on_select(move || state.toggle_mute(mute.clone())),
            crate::sidebar::circle_membership_item(state.circles, row.clone()),
            MenuItem::new(i18n.tr("Delete chat…", "删除会话…"))
                .on_select(move || state.remove(row.clone())),
        ]
    })
}

pub(crate) fn surface(state: State) -> impl IntoElement {
    let presence = crate::motion::Presence::modal(move || state.muting.with(Option::is_some));
    let i18n = i18n::current();
    view()
        .child(when(state.failed, move || {
            // Over the panes: this surface is a sibling of the home columns.
            view()
                .absolute()
                .right_4()
                .bottom_4()
                .max_w_80()
                .px_4()
                .py_2()
                .flex_row()
                .items_center()
                .gap_2()
                .rounded(ui::radius_md())
                .bg(ui::surface_popover)
                .border_1()
                .border_color(ui::border)
                .shadow_lg()
                .child(ui::error_text(move || i18n.text(Key::CommandFailed)).flex_1())
                .child(ui::action(
                    move || i18n.tr("Dismiss", "关闭"),
                    move || state.failed.set(false),
                ))
        }))
        .child(when(
            move || presence.mounted(),
            move || mute_dialog(state, presence),
        ))
}

fn mute_dialog(state: State, presence: crate::motion::Presence) -> impl IntoElement {
    let i18n = i18n::current();
    let duration = signal(None::<i64>);
    let confirm = move || {
        let (Some(row), Some(seconds)) = (state.muting.get_untracked(), duration.get_untracked())
        else {
            return;
        };
        let account = state.account.with(|account| account.clone());
        state.busy.set(true);
        spawn(async move {
            let result = runtime::commit(async move {
                account
                    .conversation()
                    .set_muted(row.conversation_id, row.owner_id, row.category, seconds)
                    .await
            })
            .await;
            state.busy.set(false);
            state.failed.set(result.is_err());
            if result.is_ok() {
                state.muting.set(None);
            }
        });
    };
    ui::modal(
        presence,
        move || {
            if !state.busy.get_untracked() {
                state.muting.set(None);
            }
        },
        ui::modal_card()
            .child(ui::modal_title(move || {
                i18n.tr("Mute notifications", "通知静音")
            }))
            .child(
                ui::card().children(
                    [
                        (3600, "1 hour", "1 小时"),
                        (28800, "8 hours", "8 小时"),
                        (604800, "1 week", "1 周"),
                        (31536000, "1 year", "1 年"),
                    ]
                    .into_iter()
                    .map(move |(seconds, en, zh)| {
                        ui::settings_row(None, move || i18n.tr(en, zh))
                            .selected(move || duration.get() == Some(seconds))
                            .disabled(state.busy)
                            .on_press(move || duration.set(Some(seconds)))
                    }),
                ),
            )
            .child(
                parts::dialog_actions()
                    .child(
                        ui::action(
                            move || i18n.text(Key::Cancel),
                            move || state.muting.set(None),
                        )
                        .disabled(state.busy),
                    )
                    .child(
                        ui::primary()
                            .disabled(move || duration.get().is_none() || state.busy.get())
                            .child(text(move || i18n.tr("Mute", "静音")))
                            .on_press(confirm),
                    ),
            ),
    )
}
