use std::{collections::HashMap, future::Future, sync::Arc};

use mixin_desktop_api::{
    AccountClient, AudioPlaybackSnapshot, ClientResult, MediaClient, MessageActions,
    MessageListView,
};
use nsh_base::alert_dialog;
use nsh_core::{element::desktop::MenuItem, file_dialog::SaveDialog, prelude::*};

use super::message_content::{available, is_attachment};
use super::reedit::Reedit;
use crate::{
    i18n::{self, I18n, Key},
    runtime, ui,
};

#[derive(Clone)]
pub(super) enum Detail {
    Message(Box<MessageListView>),
    User(String),
    Qr(String),
    Image {
        message: Box<MessageListView>,
        transcript: Option<String>,
    },
    Markdown {
        message: Box<MessageListView>,
        transcript: Option<String>,
    },
    AddSticker(Box<MessageListView>),
    Sticker(String),
    TranscriptSummary {
        transcript_id: String,
        child_message_id: String,
    },
}

#[derive(Clone, Copy)]
pub(super) struct Actions {
    pub owner: Owner,
    pub account: Account,
    pub i18n: I18n,
    pub quote: Signal<Option<MessageListView>>,
    pub text_selection: super::message_text::Selection,
    pub can_send: Signal<bool>,
    pub selected: Signal<Vec<String>>,
    pub forwarding: Signal<Vec<String>>,
    pub combine: Signal<bool>,
    pub policies: Signal<HashMap<String, MessageActions>>,
    pub failed: Signal<bool>,
    pub busy: Signal<bool>,
    pub pinned_page: bool,
    pub media: Stored<MediaClient>,
    pub playback: Signal<AudioPlaybackSnapshot>,
    pub preview: Signal<Option<Detail>>,
    pub preview_history: Signal<Vec<Detail>>,
    pub toasts: ui::Toasts,
}

/// The account a surface's messages belong to. Rows read only its id while they
/// render; commands take its client.
#[derive(Clone, Copy)]
pub(super) struct Account {
    client: Stored<Option<Arc<AccountClient>>>,
    id: Stored<String>,
}

impl Account {
    pub fn new(client: Arc<AccountClient>) -> Self {
        let id = client.account_id();
        Self {
            client: stored(Some(client)),
            id: stored(id),
        }
    }

    pub fn id(self) -> String {
        self.id.with(|id| id.clone())
    }

    /// The client for a command. Only a detached preview has none.
    pub fn with<T>(self, f: impl FnOnce(&Arc<AccountClient>) -> T) -> T {
        self.client.with(|client| {
            f(client
                .as_ref()
                .expect("a detached preview runs no account commands"))
        })
    }

    /// For a view that renders without the account, as a sticker does: it
    /// needs one only to animate or to refresh a failed asset.
    pub fn client(self) -> Option<Arc<AccountClient>> {
        self.client.with(|account| account.clone())
    }
}

impl Actions {
    /// Actions over fixed rows, for the debug preview. Rows render as in a chat;
    /// a command that needs the account is not available.
    #[cfg(debug_assertions)]
    pub fn detached(account_id: String) -> Self {
        let playback =
            use_context::<crate::playback::Playback>().expect("Preview provides playback");
        Self {
            owner: Owner::current(),
            account: Account {
                client: stored(None),
                id: stored(account_id),
            },
            i18n: i18n::current(),
            quote: signal_with(None, SignalEquals::Never),
            text_selection: super::message_text::Selection::new(),
            can_send: signal(true),
            selected: signal(Vec::new()),
            forwarding: signal(Vec::new()),
            combine: signal(false),
            policies: signal_with(HashMap::new(), SignalEquals::Never),
            failed: signal(false),
            busy: signal(false),
            pinned_page: false,
            media: stored(crate::media_client().as_ref().clone()),
            playback: playback.snapshot_signal(),
            preview: signal_with(None, SignalEquals::Never),
            preview_history: signal_with(Vec::new(), SignalEquals::Never),
            toasts: use_context::<ui::Toasts>().expect("Preview provides toasts"),
        }
    }

    /// Runs under the surface, not the caller: a menu closes and a list cell is
    /// recycled before the command answers, and either would end the task with
    /// `busy` still set.
    pub fn task(self, future: impl Future<Output = ()> + 'static) {
        self.owner.run(|| {
            spawn(future);
        });
    }

    /// Reports a committed command's outcome, which outlives this surface.
    pub fn commit(self, command: impl Future<Output = ClientResult<()>> + Send + 'static) {
        let command = runtime::commit(command);
        self.task(async move {
            self.finish(command.await.is_ok());
        });
    }

    /// As [`Self::commit`], confirming success with a notice.
    pub fn commit_with(
        self,
        command: impl Future<Output = ClientResult<()>> + Send + 'static,
        done: &'static str,
    ) {
        let command = runtime::commit(command);
        self.task(async move {
            let ok = command.await.is_ok();
            self.finish(ok);
            if ok {
                self.toasts.success(done);
            }
        });
    }

    /// A command outside a dialog reports a failure as a notice, which does not stay.
    fn finish(self, ok: bool) {
        self.failed.set(false);
        if !ok {
            self.toasts.error(self.i18n.text(Key::CommandFailed));
        }
    }

    /// As [`Self::commit`], for commands that exclude each other.
    fn commit_exclusive(self, command: impl Future<Output = ClientResult<()>> + Send + 'static) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        let command = runtime::commit(command);
        self.task(async move {
            self.finish(command.await.is_ok());
            self.busy.set(false);
        });
    }

    fn open(self, detail: Detail) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(detail));
    }

    pub fn open_text_qr(self, content: String) {
        self.open(Detail::Qr(content));
    }

    pub fn open_sticker(self, sticker_id: String) {
        self.open(Detail::Sticker(sticker_id));
    }

    pub fn open_transcript_summary(self, transcript_id: String, child_message_id: String) {
        self.open(Detail::TranscriptSummary {
            transcript_id,
            child_message_id,
        });
    }

    pub fn open_preview(self, row: MessageListView) {
        self.open(Detail::Message(Box::new(row)));
    }

    pub fn open_image(self, row: MessageListView, transcript: Option<String>) {
        self.open(Detail::Image {
            message: Box::new(row),
            transcript,
        });
    }

    pub fn open_markdown(self, row: MessageListView, transcript: Option<String>) {
        self.open(Detail::Markdown {
            message: Box::new(row),
            transcript,
        });
    }

    pub fn show_user(self, id: String) {
        self.open(Detail::User(id));
    }

    pub fn add_image_sticker(self, row: MessageListView) {
        self.open(Detail::AddSticker(Box::new(row)));
    }

    pub fn close_preview(self) {
        self.preview.set(None);
        self.preview_history.set(Vec::new());
    }

    pub fn back_preview(self) {
        let mut previous = None;
        self.preview_history
            .update(|history| previous = history.pop());
        self.preview.set(previous);
    }
}

pub(super) fn attachment_name(row: &MessageListView) -> String {
    row.media_name
        .as_deref()
        .and_then(|name| name.rsplit(['/', '\\']).next())
        .filter(|name| !name.trim().is_empty() && !matches!(*name, "." | ".."))
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let extension = if row.category.ends_with("_IMAGE") {
                match row.media_mime_type.as_deref() {
                    Some("image/png") => ".png",
                    Some("image/gif") => ".gif",
                    Some("image/webp") => ".webp",
                    _ => ".jpg",
                }
            } else {
                ""
            };
            format!("{}{extension}", row.message_id)
        })
}

pub(super) fn item(
    icon: ui::MenuIcon,
    title: &'static str,
    action: impl Fn() + 'static,
) -> MenuItem {
    MenuItem::new(title).icon(icon.src()).on_select(action)
}

/// `body` hosts the menu itself, so it is the view the menu opens over.
pub(super) fn menu<E: Interactive>(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
    body: E,
) -> E {
    menu_with_context(message, actions, body, false, |_| {})
}

pub(super) fn pinned_menu<E: Interactive>(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
    body: E,
    locate: impl Fn(String) + Copy + 'static,
) -> E {
    menu_with_context(message, actions, body, true, locate)
}

fn menu_with_context<E: Interactive>(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
    body: E,
    pinned_page: bool,
    locate: impl Fn(String) + Copy + 'static,
) -> E {
    let i18n = i18n::current();
    let reedit = use_context::<Reedit>().expect("Home provides recall editing");
    // The menu snapshots its object on opening. Recycled list cells must not
    // retarget an already open destructive command to another message.
    ui::context_menu_on(body, move || {
        if actions.busy.get_untracked() || !actions.selected.with_untracked(Vec::is_empty) {
            return Vec::new();
        }
        let Some(row) = message() else {
            return Vec::new();
        };
        let policy = actions
            .policies
            .with_untracked(|p| p.get(&row.message_id).cloned());
        let mut reply = Vec::new();
        if !pinned_page && actions.can_send.get() && policy.as_ref().is_some_and(|p| p.can_reply) {
            let quoted = row.clone();
            reply.push(item(
                ui::MenuIcon::Reply,
                i18n.tr("Reply", "回复"),
                move || {
                    if actions.can_send.get_untracked() {
                        actions.quote.set(Some(quoted.clone()));
                    }
                },
            ));
        }
        let mut copy = Vec::new();
        if row.category.ends_with("_IMAGE") {
            let id = row.message_id.clone();
            copy.push(
                item(
                    ui::MenuIcon::Copy,
                    i18n.tr("Copy Image", "复制图片"),
                    move || {
                        if actions.busy.get_untracked() {
                            return;
                        }
                        let id = id.clone();
                        let client = actions.account.with(|a| a.clone());
                        actions.busy.set(true);
                        actions.task(async move {
                            match runtime::run(async move {
                                client
                                    .attachment()
                                    .attachment_image_data_uri(id, None)
                                    .await
                            })
                            .await
                            {
                                Ok(uri) => {
                                    set_clipboard_image(uri);
                                    actions
                                        .toasts
                                        .success(i18n.tr("Image copied", "图片已复制"));
                                }
                                Err(_) => actions.finish(false),
                            }
                            actions.busy.set(false);
                        });
                    },
                )
                .disabled(!available(&row)),
            );
        }
        let readable = !matches!(row.status.as_str(), "UNKNOWN" | "FAILED")
            && super::message_system::supported(&row.category);
        if readable && (row.category.ends_with("_TEXT") || row.category.ends_with("_POST")) {
            let selected = if row.category.ends_with("_TEXT") {
                actions.text_selection.selected(&row.message_id, None)
            } else {
                None
            };
            let has_selection = selected.is_some();
            let content = selected.unwrap_or_else(|| row.content.clone());
            let qr_content = content.clone();
            copy.push(item(
                ui::MenuIcon::Copy,
                if has_selection {
                    i18n.tr("Copy Selected Text", "复制已选择的文本")
                } else {
                    i18n.tr("Copy", "复制")
                },
                move || {
                    set_clipboard(content.clone());
                    actions.toasts.success(i18n.tr("Copied", "已复制"));
                },
            ));
            if row.category.ends_with("_TEXT") && !row.content.is_empty() {
                copy.push(item(
                    ui::MenuIcon::QrCode,
                    i18n.tr("Generate QR Code", "生成二维码"),
                    move || {
                        actions.open_text_qr(qr_content.clone());
                    },
                ));
            }
        } else if let Some(caption) = row.caption.clone().filter(|s| readable && !s.is_empty()) {
            let selected = actions.text_selection.selected(&row.message_id, None);
            let has_selection = selected.is_some();
            let caption = selected.unwrap_or(caption);
            copy.push(item(
                ui::MenuIcon::Copy,
                if has_selection {
                    i18n.tr("Copy Selected Text", "复制已选择的文本")
                } else {
                    i18n.tr("Copy Text", "复制文字")
                },
                move || {
                    set_clipboard(caption.clone());
                    actions.toasts.success(i18n.tr("Copied", "已复制"));
                },
            ));
        }
        let mut message_group = Vec::new();
        if pinned_page {
            let id = row.message_id.clone();
            message_group.push(item(
                ui::MenuIcon::PositionToChat,
                i18n.tr("Locate in Chat", "前往聊天"),
                move || locate(id.clone()),
            ));
        }
        if policy.as_ref().is_some_and(|p| p.can_forward) {
            let id = row.message_id.clone();
            message_group.push(item(
                ui::MenuIcon::Forward,
                i18n.tr("Forward", "转发"),
                move || {
                    actions.combine.set(false);
                    actions.forwarding.set(vec![id.clone()]);
                },
            ));
        }
        let id = row.message_id.clone();
        message_group.push(item(
            ui::MenuIcon::Select,
            i18n.tr("Select", "选择"),
            move || {
                actions.selected.update(|ids| {
                    if !ids.contains(&id) {
                        ids.push(id.clone());
                    }
                })
            },
        ));
        if policy.as_ref().is_some_and(|p| p.can_pin) {
            let message_id = row.message_id.clone();
            let conversation_id = row.conversation_id.clone();
            let pin = !row.pinned;
            message_group.push(item(
                if pin {
                    ui::MenuIcon::Pin
                } else {
                    ui::MenuIcon::UnPin
                },
                i18n.tr(
                    if pin { "Pin" } else { "Unpin" },
                    if pin { "置顶" } else { "取消置顶" },
                ),
                move || {
                    let client = actions.account.with(|a| a.clone());
                    let id = message_id.clone();
                    let conversation_id = conversation_id.clone();
                    actions.commit_exclusive(async move {
                        client
                            .message()
                            .set_message_pinned(conversation_id, id, pin)
                            .await
                    });
                },
            ));
        }
        let mut save = Vec::new();
        if is_attachment(&row) && available(&row) {
            let account = actions.account;
            let id = row.message_id.clone();
            let name = attachment_name(&row);
            save.push(item(
                ui::MenuIcon::Download,
                i18n.tr("Save as", "另存为"),
                move || {
                    let id = id.clone();
                    let name = name.clone();
                    let client = account.with(|a| a.clone());
                    actions.task(async move {
                        match pick_save_path(SaveDialog::new().file_name(name)).await {
                            Ok(Some(path)) => actions.commit_with(
                                async move {
                                    client
                                        .attachment()
                                        .save_attachment_as(id, path.to_string_lossy().into_owned())
                                        .await
                                },
                                i18n.tr("Saved", "已保存"),
                            ),
                            Ok(None) => {}
                            Err(_) => actions.finish(false),
                        }
                    });
                },
            ));
        }
        let mut sticker = Vec::new();
        if row.category.ends_with("_STICKER")
            && let Some(id) = row.sticker_id.clone()
        {
            sticker.push(item(
                ui::MenuIcon::Sticker,
                i18n.tr("Add Sticker", "添加贴纸"),
                move || {
                    let client = actions.account.with(|a| a.clone());
                    let id = id.clone();
                    actions.commit_with(
                        async move { client.sticker().add_sticker(id).await },
                        i18n.tr("Sticker added", "已添加贴纸"),
                    );
                },
            ));
        }
        if row.category.ends_with("_IMAGE")
            && available(&row)
            && policy.as_ref().is_some_and(|p| p.can_forward)
        {
            let image = row.clone();
            sticker.push(item(
                ui::MenuIcon::Sticker,
                i18n.tr("Add Sticker", "添加贴纸"),
                move || {
                    actions.add_image_sticker(image.clone());
                },
            ));
        }
        let mut delete = Vec::new();
        if policy.as_ref().is_some_and(|p| p.can_recall) {
            let row = row.clone();
            let recover = (row.category.ends_with("_TEXT")
                && actions.account.id() == row.sender_id)
                .then(|| row.content.clone());
            delete.push(item(
                ui::MenuIcon::Recall,
                i18n.tr("Delete for Everyone", "撤回"),
                move || {
                    remove_with_reedit(
                        actions,
                        row.conversation_id.clone(),
                        vec![row.message_id.clone()],
                        true,
                        recover
                            .clone()
                            .map(|text| (reedit, row.message_id.clone(), text)),
                    )
                },
            ));
        }
        if !pinned_page {
            let row = row.clone();
            delete.push(item(
                ui::MenuIcon::Delete,
                i18n.tr("Delete for me", "删除"),
                move || {
                    remove(
                        actions,
                        row.conversation_id.clone(),
                        vec![row.message_id.clone()],
                        false,
                    )
                },
            ));
        }
        ui::menu_groups([reply, copy, message_group, save, sticker, delete])
    })
    .min_w_0()
    .flex_shrink(1.0)
}

pub(super) fn remove(actions: Actions, conversation_id: String, ids: Vec<String>, recall: bool) {
    remove_with_reedit(actions, conversation_id, ids, recall, None);
}

fn remove_with_reedit(
    actions: Actions,
    conversation_id: String,
    ids: Vec<String>,
    recall: bool,
    recover: Option<(Reedit, String, String)>,
) {
    if actions.busy.get_untracked() || ids.is_empty() {
        return;
    }
    let i18n = actions.i18n;
    let client = actions.account.with(|a| a.clone());
    actions.task(async move {
        let title = if recall {
            i18n.tr("Delete for everyone?", "为所有人删除消息？")
        } else {
            i18n.tr("Delete for me?", "为我删除消息？")
        };
        let result = alert_dialog()
            .title(title)
            .description(if recall {
                i18n.tr(
                    "These messages will be removed for everyone in this conversation.",
                    "这些消息将从此会话所有人的聊天记录中删除。",
                )
            } else {
                i18n.tr(
                    "These messages will be removed from this device.",
                    "这些消息将从此设备的聊天记录中删除。",
                )
            })
            .cancel("cancel", i18n.text(Key::Cancel))
            .destructive("delete", i18n.tr("Delete", "删除"))
            .show()
            .await;
        if !matches!(result, Ok(answer) if answer.action.as_deref() == Some("delete")) {
            return;
        }
        actions.busy.set(true);
        let removed = ids.clone();
        let recalled_conversation = conversation_id.clone();
        let result = runtime::commit(async move {
            if recall {
                client.message().recall_messages(conversation_id, ids).await
            } else {
                client.message().delete_messages(conversation_id, ids).await
            }
        })
        .await;
        actions.failed.set(result.is_err());
        if result.is_ok() {
            if let Some((reedit, id, text)) = recover {
                reedit.remember(id, recalled_conversation, text);
            }
            actions
                .selected
                .update(|selected| selected.retain(|id| !removed.contains(id)));
            if actions
                .quote
                .with(|q| q.as_ref().is_some_and(|m| removed.contains(&m.message_id)))
            {
                actions.quote.set(None);
            }
        }
        actions.busy.set(false);
    });
}
