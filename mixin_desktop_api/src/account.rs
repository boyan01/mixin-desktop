use std::sync::Arc;

use async_stream::stream;
use chrono::Utc;
use futures::Stream;
use mixin_desktop_core::core::conversation_change::ConversationChange;
use mixin_desktop_core::runtime::{desktop::DesktopRuntime, AccountRuntime};

use crate::{
    AccountProfile, AttachmentAccess, CircleItem, ClientResult, ConversationAccess,
    ConversationChangeEvent, ConversationListItem, ConversationStorageUsage,
    ConversationUnseenCount, DeviceTransferCommand, DeviceTransferEvent, MessageAccess,
    NotificationEvent, SnapshotDetailItem, StickerAccess, StorageCategoryUsage, UserAccess,
};

pub struct AccountClient {
    runtime: Arc<AccountRuntime>,
    desktop: Arc<DesktopRuntime>,
}

impl AccountClient {
    pub(crate) fn new(runtime: Arc<AccountRuntime>, desktop: Arc<DesktopRuntime>) -> Self {
        Self { runtime, desktop }
    }

    pub fn account_id(&self) -> String {
        self.runtime.account_id().to_string()
    }

    pub fn profile(&self) -> AccountProfile {
        AccountProfile::from(&self.runtime.account())
    }

    pub async fn update_profile(
        &self,
        full_name: String,
        biography: String,
    ) -> ClientResult<AccountProfile> {
        let account = self
            .runtime
            .update_account_profile(full_name, biography)
            .await?;
        Ok(AccountProfile::from(&account))
    }

    pub async fn update_avatar(&self, avatar_base64: String) -> ClientResult<AccountProfile> {
        let account = self.runtime.update_account_avatar(avatar_base64).await?;
        Ok(AccountProfile::from(&account))
    }

    pub async fn refresh_profile(&self) -> ClientResult<AccountProfile> {
        let account = self.runtime.refresh_account_profile().await?;
        Ok(AccountProfile::from(&account))
    }

    pub fn profile_changes(&self) -> impl Stream<Item = AccountProfile> + Send + 'static {
        let mut profile = self.runtime.subscribe_profile_changes();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            let initial = profile.borrow().clone();
            yield AccountProfile::from(&initial);
            loop {
                tokio::select! {
                    changed = profile.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        let account = profile.borrow_and_update().clone();
                        yield AccountProfile::from(&account);
                    }
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn storage_usage(&self) -> ClientResult<Vec<ConversationStorageUsage>> {
        Ok(self.runtime.storage_usage().await?)
    }

    pub async fn snapshot_by_trace(&self, trace_id: String) -> ClientResult<SnapshotDetailItem> {
        Ok(self.runtime.snapshot_by_trace(trace_id).await?)
    }

    pub async fn snapshot_by_id(&self, snapshot_id: String) -> ClientResult<SnapshotDetailItem> {
        Ok(self.runtime.snapshot_by_id(snapshot_id).await?)
    }

    pub async fn safe_snapshot_by_id(
        &self,
        snapshot_id: String,
    ) -> ClientResult<SnapshotDetailItem> {
        Ok(self.runtime.safe_snapshot_by_id(snapshot_id).await?)
    }

    pub fn media_directory(&self) -> ClientResult<String> {
        Ok(self
            .runtime
            .media_directory()?
            .to_string_lossy()
            .into_owned())
    }

    pub async fn conversation_storage_usage(
        &self,
        conversation_id: String,
    ) -> ClientResult<Vec<StorageCategoryUsage>> {
        Ok(self
            .runtime
            .conversation_storage_usage(conversation_id)
            .await?)
    }

    pub async fn clear_conversation_storage(
        &self,
        conversation_id: String,
        categories: Vec<String>,
    ) -> ClientResult<()> {
        Ok(self
            .runtime
            .clear_conversation_storage(conversation_id, categories)
            .await?)
    }

    pub fn conversation(&self) -> ConversationAccess {
        self.runtime.conversation_access().into()
    }

    pub fn message(&self) -> MessageAccess {
        self.runtime.message_access().into()
    }

    pub fn attachment(&self) -> AttachmentAccess {
        self.runtime.attachment_access().into()
    }

    pub fn attachment_progress(&self, message_id: String) -> f64 {
        self.runtime.attachment_progress(&message_id)
    }

    pub fn sticker(&self) -> StickerAccess {
        self.runtime.sticker_access().into()
    }

    pub fn user(&self) -> UserAccess {
        self.runtime.user_access().into()
    }

    pub fn conversation_changes(
        &self,
    ) -> impl Stream<Item = ConversationChangeEvent> + Send + 'static {
        let mut changes = self.runtime.subscribe_conversation_changes();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            loop {
                if *shutdown.borrow() {
                    break;
                }
                tokio::select! {
                    result = changes.recv() => {
                        let event = match result {
                            Ok(ConversationChange::Conversation(conversation_id)) => ConversationChangeEvent {
                                conversation_ids: vec![conversation_id],
                                reload_all: false,
                            },
                            Ok(ConversationChange::All) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => ConversationChangeEvent {
                                conversation_ids: Vec::new(),
                                reload_all: true,
                            },
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        };
                        yield event;
                    }
                    result = shutdown.changed() => {
                        if result.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn circle_changes(&self) -> impl Stream<Item = Vec<CircleItem>> + Send + 'static {
        let changes = self
            .runtime
            .conversation_access()
            .subscribe_circle_changes();
        stream! {
            futures::pin_mut!(changes);
            while let Some(circles) = futures::StreamExt::next(&mut changes).await {
                yield circles;
            }
        }
    }

    pub fn unseen_count_changes(
        &self,
    ) -> impl Stream<Item = Vec<ConversationUnseenCount>> + Send + 'static {
        let changes = self
            .runtime
            .conversation_access()
            .subscribe_unseen_count_changes();
        stream! {
            futures::pin_mut!(changes);
            while let Some(counts) = futures::StreamExt::next(&mut changes).await {
                yield counts;
            }
        }
    }

    pub fn unseen_message_count_changes(&self) -> impl Stream<Item = i64> + Send + 'static {
        self.runtime
            .conversation_access()
            .subscribe_unseen_message_count_changes()
    }

    pub fn message_changes(&self) -> impl Stream<Item = u64> + Send + 'static {
        let mut changes = self.runtime.subscribe_message_changes();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            loop {
                if *shutdown.borrow() {
                    break;
                }
                tokio::select! {
                    result = changes.changed() => {
                        if result.is_err() {
                            break;
                        }
                        let revision = *changes.borrow();
                        yield revision;
                    }
                    result = shutdown.changed() => {
                        if result.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn notification_events(
        &self,
    ) -> impl Stream<Item = ClientResult<NotificationEvent>> + Send + 'static {
        let runtime = self.runtime.clone();
        stream! {
            let mut changes = runtime.subscribe_notification_changes();
            let mut shutdown = runtime.subscribe_shutdown();
            let mut created_at_micros = Utc::now().timestamp_micros();
            let mut row_id = match runtime.latest_notification_row_id().await {
                Ok(row_id) => row_id,
                Err(error) => {
                    yield Err(error.into());
                    return;
                }
            };
            loop {
                loop {
                    let batch = match runtime
                        .notification_event_batch(created_at_micros, row_id, 200)
                        .await
                    {
                        Ok(batch) => batch,
                        Err(error) => {
                            yield Err(error.into());
                            return;
                        }
                    };
                    created_at_micros = batch.next_created_at_micros;
                    row_id = batch.next_row_id;
                    for event in batch.events {
                        yield Ok(event);
                    }
                    if !batch.has_more {
                        break;
                    }
                }
                tokio::select! {
                    changed = changes.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn device_transfer_events(
        &self,
    ) -> impl Stream<Item = DeviceTransferEvent> + Send + 'static {
        let mut events = self.runtime.device_transfer().subscribe();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            loop {
                tokio::select! {
                    event = events.recv() => match event {
                        Ok(event) => yield event.into(),
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn device_transfer_command(
        &self,
        command: DeviceTransferCommand,
    ) -> ClientResult<()> {
        Ok(self
            .runtime
            .device_transfer()
            .command(command.into())
            .await?)
    }

    pub fn connection_status(&self) -> impl Stream<Item = bool> + Send + 'static {
        let mut status = self.runtime.subscribe_connection_status();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            let initial = *status.borrow();
            yield initial;
            loop {
                tokio::select! {
                    changed = status.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        let connected = *status.borrow_and_update();
                        yield connected;
                    }
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn retry_connection(&self) {
        self.runtime.retry_connection();
    }

    pub fn account_health(&self) -> impl Stream<Item = String> + Send + 'static {
        let mut health = self.runtime.subscribe_account_health();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            let initial = health.borrow().clone();
            yield initial;
            loop {
                tokio::select! {
                    changed = health.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        let value = health.borrow_and_update().clone();
                        yield value;
                    }
                    changed = shutdown.changed() => {
                        if changed.is_err() || *shutdown.borrow() {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn refresh_account_health(&self) -> ClientResult<()> {
        Ok(self.runtime.refresh_account_health().await?)
    }

    pub async fn conversations(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> ClientResult<Vec<ConversationListItem>> {
        let conversations = self
            .runtime
            .conversation_access()
            .conversations(category, circle_id, keyword, unseen_only, limit, offset)
            .await?;
        Ok(conversations.into_iter().map(Into::into).collect())
    }

    pub async fn shutdown(&self) {
        self.desktop.shutdown_account(&self.runtime).await;
    }

    pub async fn sign_out(&self) -> ClientResult<()> {
        Ok(self.desktop.sign_out_account(&self.runtime).await?)
    }
}
