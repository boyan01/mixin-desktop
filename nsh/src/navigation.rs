//! Session navigation shared by messages, profiles, search, and native links.

use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::runtime;

#[derive(Clone, Copy)]
pub(crate) struct ChatNavigation {
    owner: Owner,
    selected: Signal<Option<ConversationListData>>,
    target: Signal<Option<String>>,
    settings: Signal<bool>,
    request: Signal<u64>,
    recent: Signal<Vec<String>>,
    search_highlight: Signal<Option<(String, String)>>,
    search_from_info: Signal<bool>,
    pub panel: Signal<Option<crate::inspector::Mode>>,
    pub return_target: Signal<Option<String>>,
}

impl ChatNavigation {
    pub(crate) fn new(
        selected: Signal<Option<ConversationListData>>,
        target: Signal<Option<String>>,
        settings: Signal<bool>,
    ) -> Self {
        let request = signal(0_u64);
        let recent = signal(Vec::<String>::new());
        let search_highlight = signal(None::<(String, String)>);
        let panel = signal(None);
        let search_from_info = signal(false);
        let search_origin_scope = computed(move || {
            (
                selected.with(|row| row.as_ref().map(|row| row.conversation_id.clone())),
                target.get(),
                settings.get(),
            )
        });
        effect(move || {
            let _scope = search_origin_scope.get();
            search_from_info.set(false);
        });
        effect(move || {
            if panel.get() != Some(crate::inspector::Mode::Search) {
                search_from_info.set(false);
            }
        });
        effect(move || {
            let selected_id =
                selected.with(|row| row.as_ref().map(|row| row.conversation_id.clone()));
            if search_highlight.with_untracked(|value| {
                value
                    .as_ref()
                    .is_some_and(|(id, _)| Some(id) != selected_id.as_ref())
            }) {
                search_highlight.set(None);
            }
        });
        effect(move || {
            if let Some(id) =
                selected.with(|row| row.as_ref().map(|row| row.conversation_id.clone()))
            {
                recent.update(|ids| {
                    ids.retain(|existing| existing != &id);
                    ids.insert(0, id);
                    ids.truncate(5);
                });
            }
        });
        // Invalidate a pending link when another entry point changes navigation.
        // Search, create, and native menus still own their existing selection signals.
        effect(move || {
            selected.with(|_| ());
            let _target = target.get();
            let _settings = settings.get();
            request.update(|revision| *revision += 1);
        });
        Self {
            owner: Owner::current(),
            selected,
            target,
            settings,
            request,
            recent,
            search_highlight,
            search_from_info,
            panel,
            return_target: signal(None),
        }
    }

    pub(crate) fn cancel_pending(self) {
        self.request.update(|revision| *revision += 1);
    }

    pub(crate) fn open_search_from_info(self) {
        batch(|| {
            self.search_from_info.set(true);
            self.panel.set(Some(crate::inspector::Mode::Search));
        });
    }

    pub(crate) fn back_panel(self) {
        batch(|| {
            let next = if self.panel.get_untracked() == Some(crate::inspector::Mode::Search)
                && self.search_from_info.get_untracked()
            {
                Some(crate::inspector::Mode::Info)
            } else {
                None
            };
            self.search_from_info.set(false);
            self.panel.set(next);
        });
    }

    pub(crate) fn toggle_panel(self, mode: crate::inspector::Mode) {
        if self.panel.get_untracked() == Some(mode) {
            self.back_panel();
        } else {
            batch(|| {
                self.search_from_info.set(false);
                self.panel.set(Some(mode));
            });
        }
    }

    pub(crate) fn search_term(self, conversation_id: &str) -> String {
        self.search_highlight.with(|highlight| {
            highlight
                .as_ref()
                .filter(|(id, _)| id == conversation_id)
                .map(|(_, query)| query.clone())
                .unwrap_or_default()
        })
    }

    pub(crate) fn highlight_search(self, conversation_id: String, query: String) {
        self.search_highlight
            .set(Some((conversation_id, query.trim().to_owned())));
    }

    pub(crate) fn jump_in_selected(
        self,
        conversation_id: &str,
        message_id: String,
        origin: Option<String>,
    ) -> bool {
        if !self.selected.with_untracked(|row| {
            row.as_ref()
                .is_some_and(|row| row.conversation_id == conversation_id)
        }) {
            return false;
        }
        batch(|| {
            self.cancel_pending();
            self.settings.set(false);
            self.panel.set(None);
            self.search_highlight.set(None);
            self.return_target.set(origin);
            self.target.set(Some(message_id));
        });
        true
    }

    pub(crate) fn recent_ids(self) -> Vec<String> {
        self.recent.get()
    }

    pub(crate) fn track(self, account: Arc<AccountClient>) {
        use futures::StreamExt as _;
        self.owner.run(|| {
            spawn(async move {
                let mut changes = Box::pin(runtime::observe(account.conversation_changes()));
                while let Some(event) = changes.next().await {
                    let Some(id) = self
                        .selected
                        .with(|row| row.as_ref().map(|row| row.conversation_id.clone()))
                    else {
                        continue;
                    };
                    if !event.reload_all && !event.conversation_ids.contains(&id) {
                        continue;
                    }
                    let client = account.clone();
                    let requested_id = id.clone();
                    if let Ok(rows) = runtime::run(async move {
                        client
                            .conversation()
                            .conversation_items_by_ids(vec![requested_id])
                            .await
                    })
                    .await
                        && rows.is_empty()
                        && self
                            .selected
                            .with(|row| row.as_ref().is_some_and(|row| row.conversation_id == id))
                    {
                        self.selected.set(None);
                        self.target.set(None);
                        self.panel.set(None);
                        self.return_target.set(None);
                    }
                }
            });
        });
    }

    pub(crate) fn select(self, conversation: ConversationListData) {
        batch(|| {
            self.request.update(|revision| *revision += 1);
            self.target.set(None);
            self.panel.set(None);
            self.return_target.set(None);
            self.search_highlight.set(None);
            self.settings.set(false);
            self.selected.set(Some(conversation));
        });
    }

    pub(crate) fn open_conversation(
        self,
        account: Arc<AccountClient>,
        conversation_id: String,
        target_message_id: Option<String>,
        failed: Signal<bool>,
    ) {
        self.open_message(account, conversation_id, target_message_id, None, failed);
    }

    pub(crate) fn open_search_message(
        self,
        account: Arc<AccountClient>,
        conversation_id: String,
        message_id: String,
        query: String,
        failed: Signal<bool>,
    ) {
        self.open_message(
            account,
            conversation_id,
            Some(message_id),
            Some(query),
            failed,
        );
    }

    fn open_message(
        self,
        account: Arc<AccountClient>,
        conversation_id: String,
        target_message_id: Option<String>,
        search_term: Option<String>,
        failed: Signal<bool>,
    ) {
        self.request.update(|revision| *revision += 1);
        let request = self.request.get_untracked();
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::run(async move {
                    let row = account
                        .conversation()
                        .conversation_items_by_ids(vec![conversation_id])
                        .await?
                        .into_iter()
                        .next()
                        .ok_or(mixin_desktop_api::ClientError::NotFound)?;
                    // A deleted message must not leave the chat waiting on a missing target.
                    let target = if let Some(id) = target_message_id {
                        account
                            .message()
                            .message_items_by_ids(vec![id])
                            .await?
                            .into_iter()
                            .find(|message| message.conversation_id == row.conversation_id)
                            .map(|message| message.message_id)
                    } else {
                        None
                    };
                    Ok((row, target))
                })
                .await;
                if self.request.get_untracked() != request {
                    return;
                }
                match result {
                    Ok((row, target)) => batch(|| {
                        self.settings.set(false);
                        self.panel.set(None);
                        self.return_target.set(None);
                        self.search_highlight.set(
                            search_term
                                .filter(|_| target.is_some())
                                .map(|term| (row.conversation_id.clone(), term)),
                        );
                        self.selected.set(Some(row));
                        self.target.set(target);
                    }),
                    Err(_) => failed.set(true),
                }
            });
        });
    }

    pub(crate) fn open_user(
        self,
        account: Arc<AccountClient>,
        user_id: String,
        failed: Signal<bool>,
    ) {
        self.open_user_then(account, user_id, failed, || {});
    }

    pub(crate) fn open_user_then(
        self,
        account: Arc<AccountClient>,
        user_id: String,
        failed: Signal<bool>,
        opened: impl FnOnce() + 'static,
    ) {
        self.request.update(|revision| *revision += 1);
        let request = self.request.get_untracked();
        self.owner.run(|| {
            spawn(async move {
                let result = runtime::commit(async move {
                    let access = account.conversation();
                    let id = access.open_user_conversation(user_id).await?;
                    access
                        .conversation_items_by_ids(vec![id])
                        .await?
                        .into_iter()
                        .next()
                        .ok_or(mixin_desktop_api::ClientError::NotFound)
                })
                .await;
                if self.request.get_untracked() != request {
                    return;
                }
                match result {
                    Ok(row) => {
                        self.select(row);
                        opened();
                    }
                    Err(_) => failed.set(true),
                }
            });
        });
    }
}
