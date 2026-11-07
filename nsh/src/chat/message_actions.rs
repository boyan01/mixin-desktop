use std::{collections::HashMap, sync::Arc};

use mixin_desktop_api::{
    AccountClient, AudioPlaybackSnapshot, MediaClient, MessageActions, MessageListView,
};
use nsh_base::{alert_dialog, menu_item, theme};
use nsh_core::{file_dialog::SaveDialog, prelude::*};

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
    pub account: Stored<Arc<AccountClient>>,
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
}

impl Actions {
    pub fn open_text_qr(self, content: String) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::Qr(content)));
    }

    pub fn open_sticker(self, sticker_id: String) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::Sticker(sticker_id)));
    }

    pub fn open_transcript_summary(self, transcript_id: String, child_message_id: String) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::TranscriptSummary {
            transcript_id,
            child_message_id,
        }));
    }

    pub fn open_preview(self, row: MessageListView) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::Message(Box::new(row))));
    }
    pub fn open_image(self, row: MessageListView, transcript: Option<String>) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::Image {
            message: Box::new(row),
            transcript,
        }));
    }
    pub fn open_markdown(self, row: MessageListView, transcript: Option<String>) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::Markdown {
            message: Box::new(row),
            transcript,
        }));
    }
    pub fn show_user(self, id: String) {
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::User(id)));
    }
    pub fn add_image_sticker(self, row: MessageListView) {
        self.failed.set(false);
        if let Some(current) = self.preview.get_untracked() {
            self.preview_history.update(|history| history.push(current));
        }
        self.preview.set(Some(Detail::AddSticker(Box::new(row))));
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

pub(super) fn label(i18n: I18n, english: &'static str, chinese: &'static str) -> &'static str {
    i18n.tr(english, chinese)
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

pub(super) fn item(title: &'static str, action: impl Fn() + 'static) -> impl IntoElement {
    menu_item()
        .px_3()
        .py_2()
        .rounded_md()
        .hover(|s| s.bg(move || theme().accent))
        .focus(|s| s.bg(move || theme().accent))
        .on_select(action)
        .child(text(title).text_sm())
}

pub(super) fn menu(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
    body: impl IntoElement + 'static,
) -> impl IntoElement {
    menu_with_context(message, actions, body, false, |_| {})
}

pub(super) fn pinned_menu(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
    body: impl IntoElement + 'static,
    locate: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    menu_with_context(message, actions, body, true, locate)
}

fn menu_with_context(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
    body: impl IntoElement + 'static,
    pinned_page: bool,
    locate: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let i18n = i18n::current();
    let reedit = use_context::<Reedit>().expect("Home provides recall editing");
    // The popup snapshots its object on opening. Recycled list cells must not
    // retarget an already open destructive command to another message.
    ui::context_menu()
        .min_w_0()
        .flex_shrink(1.0)
        .disabled(move || actions.busy.get() || !actions.selected.with(Vec::is_empty))
        .trigger(body)
        .child(dynamic(move || {
            let row = untracked(message);
            let Some(row) = row else {
                return view().hidden().into_element();
            };
            let policy = untracked(|| actions.policies.with(|p| p.get(&row.message_id).cloned()));
            let mut items = Vec::<AnyElement>::new();
            if pinned_page {
                let id = row.message_id.clone();
                items.push(
                    item(
                        label(i18n, "Locate in chat", "在聊天中定位"),
                        move || locate(id.clone()),
                    )
                    .into_element(),
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
                items.push(
                    item(
                        if has_selection {
                            label(i18n, "Copy selected text", "复制选中文字")
                        } else {
                            label(i18n, "Copy", "复制")
                        },
                        move || set_clipboard(content.clone()),
                    )
                    .into_element(),
                );
                if row.category.ends_with("_TEXT") && !row.content.is_empty() {
                    items.push(
                        item(
                            label(i18n, "Generate QR code", "生成二维码"),
                            move || {
                                actions.open_text_qr(qr_content.clone());
                            },
                        )
                        .into_element(),
                    );
                }
            } else if let Some(caption) = row.caption.clone().filter(|s| readable && !s.is_empty())
            {
                let selected = actions.text_selection.selected(&row.message_id, None);
                let has_selection = selected.is_some();
                let caption = selected.unwrap_or(caption);
                items.push(
                    item(
                        if has_selection {
                            label(i18n, "Copy selected text", "复制选中文字")
                        } else {
                            label(i18n, "Copy text", "复制文字")
                        },
                        move || set_clipboard(caption.clone()),
                    )
                    .into_element(),
                );
            }
            if !pinned_page
                && actions.can_send.get()
                && policy.as_ref().is_some_and(|p| p.can_reply)
            {
                let reply = row.clone();
                items.push(
                    item(label(i18n, "Reply", "回复"), move || {
                        if actions.can_send.get_untracked() {
                            actions.quote.set(Some(reply.clone()));
                        }
                    })
                    .into_element(),
                );
            }
            if policy.as_ref().is_some_and(|p| p.can_forward) {
                let id = row.message_id.clone();
                items.push(
                    item(label(i18n, "Forward…", "转发…"), move || {
                        actions.combine.set(false);
                        actions.forwarding.set(vec![id.clone()]);
                    })
                    .into_element(),
                );
            }
            let id = row.message_id.clone();
            items.push(
                item(label(i18n, "Select", "选择"), move || {
                    actions.selected.update(|ids| {
                        if !ids.contains(&id) {
                            ids.push(id.clone());
                        }
                    })
                })
                .into_element(),
            );
            if policy.as_ref().is_some_and(|p| p.can_pin) {
                let message_id = row.message_id.clone();
                let conversation_id = row.conversation_id.clone();
                let pin = !row.pinned;
                items.push(
                    item(
                        label(
                            i18n,
                            if pin { "Pin" } else { "Unpin" },
                            if pin { "置顶" } else { "取消置顶" },
                        ),
                        move || {
                            if actions.busy.get_untracked() {
                                return;
                            }
                            let client = actions.account.with(|a| a.clone());
                            let id = message_id.clone();
                            let conversation_id = conversation_id.clone();
                            actions.busy.set(true);
                            spawn(async move {
                                actions.failed.set(
                                    runtime::commit(async move {
                                        client
                                            .message()
                                            .set_message_pinned(conversation_id.clone(), id, pin)
                                            .await
                                    })
                                    .await
                                    .is_err(),
                                );
                                actions.busy.set(false);
                            });
                        },
                    )
                    .into_element(),
                );
            }
            if is_attachment(&row) && available(&row) {
                if row.category.ends_with("_IMAGE") {
                    let id = row.message_id.clone();
                    items.push(
                        item(label(i18n, "Copy image", "复制图片"), move || {
                            if actions.busy.get_untracked() {
                                return;
                            }
                            let id = id.clone();
                            let client = actions.account.with(|a| a.clone());
                            actions.busy.set(true);
                            spawn(async move {
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
                                        actions.failed.set(false);
                                    }
                                    Err(_) => actions.failed.set(true),
                                }
                                actions.busy.set(false);
                            });
                        })
                        .into_element(),
                    );
                }
                let account = actions.account;
                let id = row.message_id.clone();
                let name = attachment_name(&row);
                items.push(
                    item(label(i18n, "Save as…", "另存为…"), move || {
                        let id = id.clone();
                        let name = name.clone();
                        let client = account.with(|a| a.clone());
                        spawn(async move {
                            match pick_save_path(SaveDialog::new().file_name(name)).await {
                                Ok(Some(path)) => actions.failed.set(
                                    runtime::commit(async move {
                                        client
                                            .attachment()
                                            .save_attachment_as(
                                                id,
                                                path.to_string_lossy().into_owned(),
                                            )
                                            .await
                                    })
                                    .await
                                    .is_err(),
                                ),
                                Ok(None) => {}
                                Err(_) => actions.failed.set(true),
                            }
                        });
                    })
                    .into_element(),
                );
            }
            if row.category.ends_with("_STICKER")
                && let Some(id) = row.sticker_id.clone()
            {
                items.push(
                    item(label(i18n, "Add sticker", "添加贴纸"), move || {
                        let client = actions.account.with(|a| a.clone());
                        let id = id.clone();
                        spawn(async move {
                            actions.failed.set(
                                runtime::commit(
                                    async move { client.sticker().add_sticker(id).await },
                                )
                                .await
                                .is_err(),
                            );
                        });
                    })
                    .into_element(),
                );
            }
            if row.category.ends_with("_IMAGE")
                && available(&row)
                && policy.as_ref().is_some_and(|p| p.can_forward)
            {
                let image = row.clone();
                items.push(
                    item(label(i18n, "Add sticker", "添加贴纸"), move || {
                        actions.add_image_sticker(image.clone());
                    })
                    .into_element(),
                );
            }
            if policy.as_ref().is_some_and(|p| p.can_recall) {
                let row = row.clone();
                let recover = (row.category.ends_with("_TEXT")
                    && actions.account.with(|account| account.account_id()) == row.sender_id)
                    .then(|| row.content.clone());
                items.push(
                    item(
                        label(i18n, "Delete for everyone…", "为所有人删除…"),
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
                    )
                    .into_element(),
                );
            }
            if !pinned_page {
                let row = row.clone();
                items.push(
                    item(
                        label(i18n, "Delete for me…", "为我删除…"),
                        move || {
                            remove(
                                actions,
                                row.conversation_id.clone(),
                                vec![row.message_id.clone()],
                                false,
                            )
                        },
                    )
                    .into_element(),
                );
            }
            view().children(items).into_element()
        }))
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
    spawn(async move {
        let title = if recall {
            label(i18n, "Delete for everyone?", "为所有人删除消息？")
        } else {
            label(i18n, "Delete for me?", "为我删除消息？")
        };
        let result = alert_dialog()
            .title(title)
            .description(if recall {
                label(
                    i18n,
                    "These messages will be removed for everyone in this conversation.",
                    "这些消息将从此会话所有人的聊天记录中删除。",
                )
            } else {
                label(
                    i18n,
                    "These messages will be removed from this device.",
                    "这些消息将从此设备的聊天记录中删除。",
                )
            })
            .cancel("cancel", i18n.text(Key::Cancel))
            .destructive("delete", label(i18n, "Delete", "删除"))
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
