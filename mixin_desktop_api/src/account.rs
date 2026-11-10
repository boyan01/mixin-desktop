#[cfg(not(target_arch = "wasm32"))]
use crate::{DeviceTransferCommand, DeviceTransferEvent};
use std::sync::Arc;

use async_stream::stream;
use chrono::Utc;
use futures::Stream;
use mixin_desktop_core::core::conversation_change::ConversationChange;
use mixin_desktop_core::runtime::{desktop::DesktopRuntime, AccountRuntime};

use crate::{
    AccountProfile, AttachmentAccess, CircleItem, ClientResult, ConversationAccess,
    ConversationChangeEvent, ConversationListItem, ConversationStorageUsage,
    ConversationUnseenCount, MessageAccess, NotificationEvent, SnapshotDetailItem, StickerAccess,
    StorageCategoryUsage, UserAccess,
};

pub struct AccountClient {
    runtime: Arc<AccountRuntime>,
    desktop: Arc<DesktopRuntime>,
}

impl AccountClient {
    pub async fn quick_search(
        &self,
        query: String,
        offset: i64,
        limit: i64,
    ) -> ClientResult<Vec<crate::QuickSearchItem>> {
        Ok(self.runtime.quick_search(query, offset, limit).await?)
    }

    pub(crate) fn new(runtime: Arc<AccountRuntime>, desktop: Arc<DesktopRuntime>) -> Self {
        Self { runtime, desktop }
    }

    pub fn screen_lock(&self) -> crate::ScreenLockClient {
        crate::ScreenLockClient {
            runtime: self.runtime.clone(),
        }
    }

    pub fn bot(&self) -> crate::BotClient {
        crate::BotClient::new(self.runtime.bot_access())
    }

    pub fn account_id(&self) -> String {
        self.runtime.account_id().to_string()
    }

    pub async fn sticker_animation(
        &self,
        url: String,
        pixels: u32,
    ) -> ClientResult<crate::StickerAnimationClient> {
        use mixin_desktop_media::sticker::{cache_source, cached_source, StickerAnimation};
        if !self.runtime.is_running() {
            return Err(crate::ClientError::Cancelled);
        }
        let source = match cached_source(&url) {
            Some(source) => source,
            None => self
                .runtime
                .sticker_access()
                .animation_source(url.clone())
                .await?
                .into(),
        };
        let animation = StickerAnimation::load(url.clone(), source.clone(), pixels).await?;
        cache_source(url, source)?;
        Ok(crate::StickerAnimationClient(animation))
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

    /// Returns cached or downloaded text content for a known text inscription.
    pub async fn inscription_content(&self, hash: String) -> ClientResult<Option<String>> {
        Ok(self.runtime.inscription_content(hash).await?)
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

    /// Live byte progress snapshots. Dropping the stream cancels the subscription.
    /// The first item includes active transfers; slow consumers receive the latest state.
    pub fn attachment_progress_changes(
        &self,
    ) -> impl Stream<Item = Vec<crate::AttachmentTransferProgress>> + Send + 'static {
        self.runtime.attachment_progress_changes()
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
                            Ok(ConversationChange::Messages(_)) => continue,
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

    /// Fires when messages change in `conversation_id`, or in any conversation when `None`.
    pub fn message_changes(
        &self,
        conversation_id: Option<String>,
    ) -> impl Stream<Item = ()> + Send + 'static {
        let mut changes = self.runtime.subscribe_conversation_changes();
        let mut shutdown = self.runtime.subscribe_shutdown();
        stream! {
            loop {
                if *shutdown.borrow() {
                    break;
                }
                tokio::select! {
                    result = changes.recv() => {
                        let relevant = match result {
                            Ok(change) => change.affects_messages(conversation_id.as_deref()),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => true,
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        };
                        if relevant {
                            yield ();
                        }
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

    /// Initial/current state followed by coalesced updates. Dropping the stream
    /// cancels observation; account shutdown or service closure ends it.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn device_transfer_states(
        &self,
    ) -> impl Stream<Item = crate::DeviceTransferSnapshot> + Send + 'static {
        crate::device_transfer::states(
            self.runtime.device_transfer().states(),
            self.runtime.subscribe_shutdown(),
        )
    }

    /// Compatibility event projection. Progress may coalesce, while current
    /// terminal phases survive slow consumers without replaying old approvals.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn device_transfer_events(
        &self,
    ) -> impl Stream<Item = DeviceTransferEvent> + Send + 'static {
        crate::device_transfer::events(
            self.runtime.device_transfer().states(),
            self.runtime.subscribe_shutdown(),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn device_transfer_request_command(
        &self,
        command: DeviceTransferCommand,
        request_id: u64,
    ) -> ClientResult<()> {
        Ok(self
            .runtime
            .device_transfer()
            .request_command(command.into(), request_id)
            .await?)
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    // Browser builds cannot listen on or connect to local sockets.
    #[cfg(target_arch = "wasm32")]
    pub fn device_transfer_states(
        &self,
    ) -> impl Stream<Item = crate::DeviceTransferSnapshot> + Send + 'static {
        futures::stream::empty()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn device_transfer_events(
        &self,
    ) -> impl Stream<Item = crate::DeviceTransferEvent> + Send + 'static {
        futures::stream::empty()
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn device_transfer_request_command(
        &self,
        _command: crate::DeviceTransferCommand,
        _request_id: u64,
    ) -> ClientResult<()> {
        Err(crate::ClientError::Internal(
            "device transfer is not supported in the browser".into(),
        ))
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn device_transfer_command(
        &self,
        _command: crate::DeviceTransferCommand,
    ) -> ClientResult<()> {
        Err(crate::ClientError::Internal(
            "device transfer is not supported in the browser".into(),
        ))
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
        account_health_changes(
            self.runtime.subscribe_account_health(),
            self.runtime.subscribe_shutdown(),
        )
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

fn account_health_changes(
    mut health: tokio::sync::watch::Receiver<String>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> impl Stream<Item = String> + Send + 'static {
    stream! {
        let mut previous = health.borrow_and_update().clone();
        yield previous.clone();
        loop {
            if *shutdown.borrow_and_update() {
                let terminal = health.borrow_and_update().clone();
                if terminal != previous { yield terminal; }
                break;
            }
            tokio::select! {
                changed = health.changed() => {
                    if changed.is_err() { break; }
                    let value = health.borrow_and_update().clone();
                    if value != previous {
                        previous = value.clone();
                        yield value;
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        let terminal = health.borrow_and_update().clone();
                        if terminal != previous { yield terminal; }
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod health_tests {
    use super::account_health_changes;
    use futures::StreamExt as _;
    use std::time::Duration;
    use tokio::sync::watch;

    #[tokio::test]
    async fn simultaneous_auth_retirement_and_shutdown_delivers_the_terminal_health_once() {
        let (health, receiver) = watch::channel("ready".into());
        let (shutdown, closing) = watch::channel(false);
        let mut updates = Box::pin(account_health_changes(receiver, closing));
        assert_eq!(updates.next().await.as_deref(), Some("ready"));
        health.send_replace("unauthorized".into());
        shutdown.send_replace(true);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), updates.next())
                .await
                .unwrap()
                .as_deref(),
            Some("unauthorized")
        );
        assert!(tokio::time::timeout(Duration::from_secs(3), updates.next())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn subscription_started_after_shutdown_yields_current_terminal_health_and_ends() {
        let (health, _) = watch::channel("unauthorized".into());
        let (shutdown, _) = watch::channel(true);
        let mut updates = Box::pin(account_health_changes(
            health.subscribe(),
            shutdown.subscribe(),
        ));
        assert_eq!(updates.next().await.as_deref(), Some("unauthorized"));
        assert!(tokio::time::timeout(Duration::from_secs(3), updates.next())
            .await
            .unwrap()
            .is_none());
    }
}
