//! Account-owned presentation entry for media shown outside the chat timeline.

use std::{collections::HashMap, sync::Arc};

use mixin_desktop_api::{AccountClient, MessageActions, MessageListView};
use nsh_core::{file_dialog::SaveDialog, prelude::*};

use super::{
    message_actions::{self, Actions, attachment_name},
    message_content, message_detail, message_forward, message_media, message_selection,
};
use crate::{i18n, runtime};

#[derive(Clone, Copy)]
pub(crate) struct MediaPreview {
    actions: Actions,
    pub failed: Signal<bool>,
}

impl MediaPreview {
    pub fn mount_mentions(self, rows: Signal<Vec<MessageListView>>) {
        super::mentions::MentionNames::mount(
            self.actions.account.with(|account| account.clone()),
            rows,
            None,
        );
    }

    pub fn new(account: Arc<AccountClient>) -> Self {
        Self::new_with_context(account, false)
    }

    pub fn pinned(account: Arc<AccountClient>) -> Self {
        Self::new_with_context(account, true)
    }

    fn new_with_context(account: Arc<AccountClient>, pinned_page: bool) -> Self {
        let media = crate::media_client().as_ref().clone();
        let failed = signal(false);
        let playback = use_context::<crate::playback::Playback>()
            .expect("Home provides playback")
            .snapshot_signal();
        Self {
            actions: Actions {
                owner: Owner::current(),
                text_selection: super::message_text::Selection::new(),
                account: super::message_actions::Account::new(account),
                i18n: i18n::current(),
                quote: signal_with(None, SignalEquals::Never),
                can_send: signal(false),
                selected: signal(Vec::new()),
                forwarding: signal(Vec::new()),
                combine: signal(false),
                policies: signal_with(HashMap::new(), SignalEquals::Never),
                failed,
                busy: signal(false),
                pinned_page,
                media: stored(media),
                playback,
                preview: signal_with(None, SignalEquals::Never),
                preview_history: signal_with(Vec::new(), SignalEquals::Never),
                toasts: use_context::<crate::ui::Toasts>().expect("Home provides toasts"),
            },
            failed,
        }
    }

    pub fn open(self, row: MessageListView) {
        if row.category.ends_with("_POST") {
            self.actions.open_preview(row);
        } else if !message_content::available(&row) {
            self.transfer(row);
        } else if row.category.ends_with("_IMAGE") {
            self.actions.open_image(row, None);
        } else if row.category.ends_with("_VIDEO") {
            self.actions.open_preview(row);
        } else if message_content::is_markdown(&row) {
            self.actions.open_markdown(row, None);
        } else if let Some(path) = row.media_url {
            self.actions.task(async move {
                self.failed.set(open_path(path).await.is_err());
            });
        }
    }

    pub fn transfer(self, row: MessageListView) {
        message_media::Transfer::of(&row, None).toggle(self.actions);
    }

    pub fn save(self, row: MessageListView) {
        let name = attachment_name(&row);
        let client = self.actions.account.with(|account| account.clone());
        self.actions.task(async move {
            match pick_save_path(SaveDialog::new().file_name(name)).await {
                Ok(Some(path)) => self.failed.set(
                    runtime::commit(async move {
                        client
                            .attachment()
                            .save_attachment_as(row.message_id, path.to_string_lossy().into_owned())
                            .await
                    })
                    .await
                    .is_err(),
                ),
                Ok(None) => {}
                Err(_) => self.failed.set(true),
            }
        });
    }

    pub fn content(self, row: MessageListView) -> AnyElement {
        message_content::body(row, self.actions, Default::default())
    }

    pub fn animation_visible(self) -> bool {
        self.actions.preview.with(Option::is_none) && self.actions.forwarding.with(Vec::is_empty)
    }

    pub fn pinned_permissions(self, policies: Vec<MessageActions>, can_send: bool) {
        self.actions.can_send.set(can_send);
        self.actions.policies.set(
            policies
                .into_iter()
                .map(|policy| (policy.message_id.clone(), policy))
                .collect(),
        );
    }

    pub fn pinned_row(
        self,
        message: impl Fn() -> Option<MessageListView> + Copy + 'static,
        body: impl IntoElement + 'static,
        locate: impl Fn(String) + Copy + 'static,
    ) -> impl IntoElement {
        let actions = self.actions;
        message_actions::pinned_menu(
            message,
            actions,
            view()
                .flex_row()
                .items_start()
                .gap_2()
                .child(when(
                    move || !actions.selected.with(Vec::is_empty),
                    move || {
                        crate::ui::command()
                            .size_8()
                            .px_1()
                            .on_press(move || {
                                if let Some(row) = message() {
                                    actions.selected.update(|ids| {
                                        if ids.contains(&row.message_id) {
                                            ids.retain(|id| id != &row.message_id);
                                        } else if ids.len() < 99 {
                                            ids.push(row.message_id);
                                        }
                                    });
                                }
                            })
                            .accessibility_label(move || {
                                Some(actions.i18n.tr("Select", "选择").into())
                            })
                            .child(text(move || {
                                if message().is_some_and(|row| {
                                    actions.selected.with(|ids| ids.contains(&row.message_id))
                                }) {
                                    "☑"
                                } else {
                                    "☐"
                                }
                            }))
                    },
                ))
                .child(body),
            locate,
        )
    }

    pub fn pinned_selection(self, conversation: Stored<String>) -> impl IntoElement {
        message_selection::pinned_selection_bar(self.actions, conversation)
    }

    pub fn selection_ids(self) -> Vec<String> {
        self.actions.selected.get()
    }

    pub fn prune_selection(self, removed: &[String]) {
        self.actions
            .selected
            .update(|selected| selected.retain(|id| !removed.contains(id)));
    }

    pub fn surface(self) -> impl IntoElement {
        view()
            .child(message_forward::forwarding(self.actions))
            .child(message_detail::preview(self.actions))
    }
}
