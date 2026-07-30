use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::{Stream, StreamExt as _};
use mixin_desktop_api::{
    AccountClient, AccountProfile, CircleItem, CodeResult, ConversationChangeEvent,
    ConversationDetailItem, ConversationListData, ConversationParticipantItem,
    ConversationStorageUsage, ConversationUnseenCount, DeviceTransferCommand,
    GroupConversationItem, ImageMessageView, NotificationEvent, SharedAppItem, SnapshotDetailItem,
    StickerDetailItem, StorageCategoryUsage, UserProfileItem,
};
use tokio::sync::{Mutex, Notify};

use crate::{
    error::SwiftClientError,
    model::{
        DeviceTransferEventItem, MessageItem, ParticipantAction, StickerAlbumSection,
        StickerLibrary,
    },
};

#[derive(uniffi::Object)]
pub struct SwiftAccountHandle {
    client: Arc<AccountClient>,
}

#[derive(uniffi::Object)]
pub struct SwiftAccountHealthSubscription {
    inner: CancellableStream<String>,
}

#[derive(uniffi::Object)]
pub struct SwiftConnectionSubscription {
    inner: CancellableStream<bool>,
}

#[derive(uniffi::Object)]
pub struct SwiftCircleSubscription {
    inner: CancellableStream<Vec<CircleItem>>,
}

#[derive(uniffi::Object)]
pub struct SwiftUnseenCountSubscription {
    inner: CancellableStream<Vec<ConversationUnseenCount>>,
}

#[derive(uniffi::Object)]
pub struct SwiftUnseenMessageCountSubscription {
    inner: CancellableStream<i64>,
}

#[derive(uniffi::Object)]
pub struct SwiftNotificationSubscription {
    inner: CancellableStream<Result<NotificationEvent, SwiftClientError>>,
}

#[derive(uniffi::Object)]
pub struct SwiftDeviceTransferSubscription {
    inner: CancellableStream<DeviceTransferEventItem>,
}

#[derive(uniffi::Object)]
pub struct SwiftConversationSubscription {
    inner: CancellableStream<ConversationChangeEvent>,
}

#[derive(uniffi::Object)]
pub struct SwiftMessageSubscription {
    inner: CancellableStream<u64>,
}

struct CancellableStream<T> {
    stream: Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>,
    cancelled: AtomicBool,
    cancel_notification: Notify,
}

impl<T> CancellableStream<T> {
    fn new(stream: impl Stream<Item = T> + Send + 'static) -> Self {
        Self {
            stream: Mutex::new(Box::pin(stream)),
            cancelled: AtomicBool::new(false),
            cancel_notification: Notify::new(),
        }
    }

    async fn next(&self) -> Option<T> {
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        let mut stream = self.stream.lock().await;
        tokio::select! {
            value = stream.next() => value,
            _ = self.cancel_notification.notified() => None,
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.cancel_notification.notify_one();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftAccountHealthSubscription {
    pub async fn next(&self) -> Option<String> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftConnectionSubscription {
    pub async fn next(&self) -> Option<bool> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftCircleSubscription {
    pub async fn next(&self) -> Option<Vec<CircleItem>> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftUnseenCountSubscription {
    pub async fn next(&self) -> Option<Vec<ConversationUnseenCount>> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftUnseenMessageCountSubscription {
    pub async fn next(&self) -> Option<i64> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftNotificationSubscription {
    pub async fn next(&self) -> Result<Option<NotificationEvent>, SwiftClientError> {
        match self.inner.next().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftDeviceTransferSubscription {
    pub async fn next(&self) -> Option<DeviceTransferEventItem> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftConversationSubscription {
    pub async fn next(&self) -> Option<ConversationChangeEvent> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftMessageSubscription {
    pub async fn next(&self) -> Option<u64> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

impl SwiftAccountHandle {
    pub fn account_id(&self) -> String {
        self.client.account_id()
    }

    pub(crate) fn new(client: AccountClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftAccountHandle {
    pub fn profile(&self) -> AccountProfile {
        self.client.profile()
    }

    pub async fn snapshot_by_id(
        &self,
        snapshot_id: String,
    ) -> Result<SnapshotDetailItem, SwiftClientError> {
        Ok(self.client.snapshot_by_id(snapshot_id).await?)
    }

    pub async fn safe_snapshot_by_id(
        &self,
        snapshot_id: String,
    ) -> Result<SnapshotDetailItem, SwiftClientError> {
        Ok(self.client.safe_snapshot_by_id(snapshot_id).await?)
    }

    pub async fn snapshot_by_trace(
        &self,
        trace_id: String,
    ) -> Result<SnapshotDetailItem, SwiftClientError> {
        Ok(self.client.snapshot_by_trace(trace_id).await?)
    }

    pub async fn resolve_code(&self, code: String) -> Result<CodeResult, SwiftClientError> {
        Ok(self.client.conversation().resolve_code(code).await?)
    }

    pub async fn join_group(&self, code: String) -> Result<String, SwiftClientError> {
        Ok(self.client.conversation().join_group(code).await?)
    }

    pub async fn set_disappearing_messages(
        &self,
        conversation_id: String,
        duration: i64,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .set_disappearing_messages(conversation_id, duration)
            .await?)
    }

    pub async fn update_profile(
        &self,
        full_name: String,
        biography: String,
    ) -> Result<AccountProfile, SwiftClientError> {
        Ok(self.client.update_profile(full_name, biography).await?)
    }

    pub async fn update_avatar(
        &self,
        avatar_base64: String,
    ) -> Result<AccountProfile, SwiftClientError> {
        Ok(self.client.update_avatar(avatar_base64).await?)
    }

    pub async fn refresh_profile(&self) -> Result<AccountProfile, SwiftClientError> {
        Ok(self.client.refresh_profile().await?)
    }

    pub fn account_health(&self) -> SwiftAccountHealthSubscription {
        SwiftAccountHealthSubscription {
            inner: CancellableStream::new(self.client.account_health()),
        }
    }

    pub fn connection_status(&self) -> SwiftConnectionSubscription {
        SwiftConnectionSubscription {
            inner: CancellableStream::new(self.client.connection_status()),
        }
    }

    pub fn retry_connection(&self) {
        self.client.retry_connection();
    }

    pub async fn refresh_account_health(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.refresh_account_health().await?)
    }

    pub fn device_transfer_events(&self) -> SwiftDeviceTransferSubscription {
        SwiftDeviceTransferSubscription {
            inner: CancellableStream::new(self.client.device_transfer_events().map(Into::into)),
        }
    }

    pub async fn device_transfer_command(
        &self,
        command: DeviceTransferCommand,
    ) -> Result<(), SwiftClientError> {
        Ok(self.client.device_transfer_command(command).await?)
    }

    pub fn media_directory(&self) -> Result<String, SwiftClientError> {
        Ok(self.client.media_directory()?)
    }

    pub async fn storage_usage(&self) -> Result<Vec<ConversationStorageUsage>, SwiftClientError> {
        Ok(self.client.storage_usage().await?)
    }

    pub async fn conversation_storage_usage(
        &self,
        conversation_id: String,
    ) -> Result<Vec<StorageCategoryUsage>, SwiftClientError> {
        Ok(self
            .client
            .conversation_storage_usage(conversation_id)
            .await?)
    }

    pub async fn clear_conversation_storage(
        &self,
        conversation_id: String,
        categories: Vec<String>,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .clear_conversation_storage(conversation_id, categories)
            .await?)
    }

    pub async fn circles(&self) -> Result<Vec<CircleItem>, SwiftClientError> {
        Ok(self.client.conversation().circles().await?)
    }

    pub fn circle_changes(&self) -> SwiftCircleSubscription {
        SwiftCircleSubscription {
            inner: CancellableStream::new(self.client.circle_changes()),
        }
    }

    pub fn unseen_count_changes(&self) -> SwiftUnseenCountSubscription {
        SwiftUnseenCountSubscription {
            inner: CancellableStream::new(self.client.unseen_count_changes()),
        }
    }

    pub fn unseen_message_count_changes(&self) -> SwiftUnseenMessageCountSubscription {
        SwiftUnseenMessageCountSubscription {
            inner: CancellableStream::new(self.client.unseen_message_count_changes()),
        }
    }

    pub fn notification_events(&self) -> SwiftNotificationSubscription {
        SwiftNotificationSubscription {
            inner: CancellableStream::new(
                self.client
                    .notification_events()
                    .map(|result| result.map_err(Into::into)),
            ),
        }
    }

    pub fn conversation_changes(&self) -> SwiftConversationSubscription {
        SwiftConversationSubscription {
            inner: CancellableStream::new(self.client.conversation_changes().map(Into::into)),
        }
    }

    pub async fn conversations(
        &self,
        category: String,
        circle_id: Option<String>,
        keyword: String,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationListData>, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .conversations(category, circle_id, keyword, unseen_only, limit, offset)
            .await?)
    }

    pub async fn conversation_items_by_ids(
        &self,
        conversation_ids: Vec<String>,
    ) -> Result<Vec<ConversationListData>, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .conversation_items_by_ids(conversation_ids)
            .await?)
    }

    pub async fn messages(
        &self,
        conversation_id: String,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<String>,
        limit: i64,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .messages(
                conversation_id,
                before_created_at_micros,
                before_message_id,
                limit,
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn message_items_by_ids(
        &self,
        message_ids: Vec<String>,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .message_items_by_ids(message_ids)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_messages(
        &self,
        conversation_id: String,
        query: String,
        sender_id: Option<String>,
        categories: Vec<String>,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .search_messages(
                conversation_id,
                query,
                sender_id,
                categories,
                anchor_message_id,
                limit,
            )
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn search_global_messages(
        &self,
        query: String,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .search_global_messages(query, anchor_message_id, limit)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn pinned_messages(
        &self,
        conversation_id: String,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .pinned_messages(conversation_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn shared_messages(
        &self,
        conversation_id: String,
        kind: String,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .shared_messages(conversation_id, kind, offset as usize, limit as usize)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn local_shared_apps(
        &self,
        user_id: String,
    ) -> Result<Vec<SharedAppItem>, SwiftClientError> {
        Ok(self.client.user().local_shared_apps(user_id).await?)
    }

    pub async fn shared_apps(
        &self,
        user_id: String,
    ) -> Result<Vec<SharedAppItem>, SwiftClientError> {
        Ok(self.client.user().shared_apps(user_id).await?)
    }

    pub async fn unread_mention_message_ids(
        &self,
        conversation_id: String,
    ) -> Result<Vec<String>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .unread_mention_message_ids(conversation_id)
            .await?)
    }

    pub async fn mark_mention_read(
        &self,
        conversation_id: String,
        message_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .message()
            .mark_mention_read(conversation_id, message_id)
            .await?)
    }

    pub async fn messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .messages_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn transcript_messages(
        &self,
        transcript_id: String,
    ) -> Result<Vec<MessageItem>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .transcript_messages(transcript_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub fn message_changes(&self) -> SwiftMessageSubscription {
        SwiftMessageSubscription {
            inner: CancellableStream::new(self.client.message_changes()),
        }
    }

    pub fn attachment_progress(&self, message_id: String) -> f64 {
        self.client.attachment_progress(message_id)
    }

    pub async fn retry_attachment(&self, message_id: String) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .attachment()
            .retry_attachment(message_id)
            .await?)
    }

    pub async fn download_attachment(&self, message_id: String) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .attachment()
            .download_attachment(message_id)
            .await?)
    }

    pub async fn cancel_attachment(&self, message_id: String) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .attachment()
            .cancel_attachment(message_id)
            .await?)
    }

    pub async fn mark_audio_read(&self, message_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.attachment().mark_audio_read(message_id).await?)
    }

    pub async fn retry_transcript_attachment(
        &self,
        transcript_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .attachment()
            .retry_transcript_attachment(transcript_id)
            .await?)
    }

    pub async fn download_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .attachment()
            .download_transcript_attachment(transcript_id, message_id)
            .await?)
    }

    pub async fn cancel_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .attachment()
            .cancel_transcript_attachment(transcript_id, message_id)
            .await?)
    }

    pub async fn mark_conversation_read(
        &self,
        conversation_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .message()
            .mark_conversation_read(conversation_id)
            .await?)
    }

    pub async fn send_text(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String, SwiftClientError> {
        if content.trim().is_empty() {
            return Err(SwiftClientError::InvalidArgument {
                message: "message content must not be empty".to_string(),
            });
        }
        if content.chars().count() > 64 * 1024 {
            return Err(SwiftClientError::InvalidArgument {
                message: "message content must not exceed 65536 characters".to_string(),
            });
        }
        Ok(self
            .client
            .message()
            .send_text(conversation_id, content, quote_message_id, silent)
            .await?)
    }

    pub async fn send_post(
        &self,
        conversation_id: String,
        content: String,
    ) -> Result<String, SwiftClientError> {
        if content.trim().is_empty() {
            return Err(SwiftClientError::InvalidArgument {
                message: "post content must not be empty".to_string(),
            });
        }
        Ok(self
            .client
            .message()
            .send_post(conversation_id, content)
            .await?)
    }

    pub async fn send_app_card(
        &self,
        conversation_id: String,
        content: String,
    ) -> Result<String, SwiftClientError> {
        if content.trim().is_empty() {
            return Err(SwiftClientError::InvalidArgument {
                message: "app card content must not be empty".to_string(),
            });
        }
        Ok(self
            .client
            .message()
            .send_app_card(conversation_id, content)
            .await?)
    }

    pub async fn image_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<ImageMessageView>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .image_messages_around(conversation_id, target_message_id, before, after)
            .await?)
    }

    pub async fn send_audio(
        &self,
        conversation_id: String,
        path: String,
        duration_millis: i64,
        waveform: Vec<u8>,
        quote_message_id: Option<String>,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .message()
            .send_audio(
                conversation_id,
                path,
                duration_millis,
                waveform,
                quote_message_id,
            )
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_attachment(
        &self,
        conversation_id: String,
        path: String,
        kind: String,
        mime_type: String,
        name: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
        duration_millis: Option<i64>,
        thumbnail: Option<String>,
        caption: Option<String>,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .message()
            .send_attachment(
                conversation_id,
                path,
                kind,
                mime_type,
                name,
                width,
                height,
                duration_millis,
                thumbnail,
                caption,
                quote_message_id,
                silent,
            )
            .await?)
    }

    pub async fn send_contact(
        &self,
        conversation_id: String,
        shared_user_id: String,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .message()
            .send_contact(conversation_id, shared_user_id, quote_message_id, silent)
            .await?)
    }

    pub async fn forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<Vec<String>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .forward_messages(target_conversation_id, source_message_ids)
            .await?)
    }

    pub async fn combine_forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .message()
            .combine_forward_messages(target_conversation_id, source_message_ids)
            .await?)
    }

    pub async fn send_sticker(
        &self,
        conversation_id: String,
        sticker_id: String,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .message()
            .send_sticker(conversation_id, sticker_id)
            .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_remote_image(
        &self,
        conversation_id: String,
        url: String,
        preview_url: String,
        width: Option<i32>,
        height: Option<i32>,
        mime_type: String,
        silent: bool,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .message()
            .send_remote_image(
                conversation_id,
                url,
                preview_url,
                width,
                height,
                mime_type,
                silent,
            )
            .await?)
    }

    pub async fn sticker_library(&self) -> Result<StickerLibrary, SwiftClientError> {
        let sticker = self.client.sticker();
        let recent = sticker.recent_stickers().await?;
        let personal = sticker.personal_stickers().await?;
        let mut sections = Vec::new();
        for album in sticker.sticker_albums().await? {
            let stickers = sticker.album_stickers(album.album_id.clone()).await?;
            sections.push(StickerAlbumSection { album, stickers });
        }
        Ok(StickerLibrary {
            recent,
            personal,
            albums: sections,
        })
    }

    pub async fn sticker_store(&self) -> Result<Vec<StickerAlbumSection>, SwiftClientError> {
        let sticker = self.client.sticker();
        let mut sections = Vec::new();
        for album in sticker.sticker_store_albums().await? {
            let stickers = sticker.album_stickers(album.album_id.clone()).await?;
            sections.push(StickerAlbumSection { album, stickers });
        }
        Ok(sections)
    }

    pub async fn refresh_stickers(&self) -> Result<bool, SwiftClientError> {
        Ok(self.client.sticker().refresh_stickers().await?)
    }

    pub async fn refresh_sticker(&self, sticker_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.sticker().refresh_sticker(sticker_id).await?)
    }

    pub async fn sticker_detail(
        &self,
        sticker_id: String,
    ) -> Result<StickerDetailItem, SwiftClientError> {
        Ok(self.client.sticker().sticker_detail(sticker_id).await?)
    }

    pub async fn set_sticker_album_added(
        &self,
        album_id: String,
        added: bool,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .sticker()
            .set_sticker_album_added(album_id, added)
            .await?)
    }

    pub async fn set_sticker_album_order(
        &self,
        album_ids: Vec<String>,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .sticker()
            .set_sticker_album_order(album_ids)
            .await?)
    }

    pub async fn add_sticker_from_path(&self, path: String) -> Result<(), SwiftClientError> {
        Ok(self.client.sticker().add_sticker_from_path(path).await?)
    }

    pub async fn add_sticker(&self, sticker_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.sticker().add_sticker(sticker_id).await?)
    }

    pub async fn add_sticker_from_file(&self, message_id: String) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .sticker()
            .add_sticker_from_file(message_id)
            .await?)
    }

    pub async fn remove_sticker(&self, sticker_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.sticker().remove_sticker(sticker_id).await?)
    }

    pub async fn current_user_role(
        &self,
        conversation_id: String,
    ) -> Result<Option<String>, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .current_user_role(conversation_id)
            .await?)
    }

    pub async fn delete_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .message()
            .delete_messages(conversation_id, message_ids)
            .await?)
    }

    pub async fn recall_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .message()
            .recall_messages(conversation_id, message_ids)
            .await?)
    }

    pub async fn set_message_pinned(
        &self,
        conversation_id: String,
        message_id: String,
        pinned: bool,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .message()
            .set_message_pinned(conversation_id, message_id, pinned)
            .await?)
    }

    pub async fn update_draft(
        &self,
        conversation_id: String,
        draft: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .update_draft(conversation_id, draft)
            .await?)
    }

    pub async fn set_conversation_pinned(
        &self,
        conversation_id: String,
        pinned: bool,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .set_pinned(conversation_id, pinned)
            .await?)
    }

    pub async fn set_conversation_muted(
        &self,
        conversation_id: String,
        owner_id: String,
        category: String,
        duration_seconds: i64,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .set_muted(conversation_id, owner_id, category, duration_seconds)
            .await?)
    }

    pub async fn delete_conversation(
        &self,
        conversation_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .delete_conversation(conversation_id)
            .await?)
    }

    pub async fn clear_conversation(
        &self,
        conversation_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .clear_conversation(conversation_id)
            .await?)
    }

    pub async fn exit_group(&self, conversation_id: String) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .exit_group(conversation_id)
            .await?)
    }

    pub async fn local_conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<ConversationDetailItem, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .local_conversation_detail(conversation_id)
            .await?)
    }

    pub async fn conversation_detail(
        &self,
        conversation_id: String,
    ) -> Result<ConversationDetailItem, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .conversation_detail(conversation_id)
            .await?)
    }

    pub async fn conversation_participants(
        &self,
        conversation_id: String,
    ) -> Result<Vec<ConversationParticipantItem>, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .conversation_participants(conversation_id)
            .await?)
    }

    pub async fn search_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<ConversationParticipantItem>, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .search_group_users(conversation_id, keyword)
            .await?)
    }

    pub async fn groups_in_common(
        &self,
        user_id: String,
    ) -> Result<Vec<GroupConversationItem>, SwiftClientError> {
        Ok(self.client.conversation().groups_in_common(user_id).await?)
    }

    pub async fn search_bot_group_users(
        &self,
        conversation_id: String,
        keyword: String,
    ) -> Result<Vec<ConversationParticipantItem>, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .search_bot_group_users(conversation_id, keyword)
            .await?)
    }

    pub async fn mention_names(
        &self,
        contents: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, SwiftClientError> {
        Ok(self.client.user().mention_names(contents).await?)
    }

    pub async fn update_participants(
        &self,
        conversation_id: String,
        action: ParticipantAction,
        user_ids: Vec<String>,
    ) -> Result<(), SwiftClientError> {
        if user_ids.is_empty() {
            return Err(SwiftClientError::InvalidArgument {
                message: "at least one participant is required".to_string(),
            });
        }
        let (action, role) = action.api_update();
        Ok(self
            .client
            .conversation()
            .update_participants(
                conversation_id,
                action.to_string(),
                user_ids,
                role.map(str::to_string),
            )
            .await?)
    }

    pub async fn edit_conversation(
        &self,
        conversation_id: String,
        name: Option<String>,
        announcement: Option<String>,
    ) -> Result<(), SwiftClientError> {
        if name.is_none() && announcement.is_none() {
            return Err(SwiftClientError::InvalidArgument {
                message: "name or announcement is required".to_string(),
            });
        }
        Ok(self
            .client
            .conversation()
            .edit_conversation(conversation_id, name, announcement)
            .await?)
    }

    pub async fn rotate_group_invite(
        &self,
        conversation_id: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .rotate_group_invite(conversation_id)
            .await?)
    }

    pub async fn user_profile(
        &self,
        user_id: String,
    ) -> Result<Option<UserProfileItem>, SwiftClientError> {
        Ok(self.client.user().user_profile(Some(user_id), None).await?)
    }

    pub async fn refresh_user_profile(
        &self,
        user_id: String,
    ) -> Result<Option<UserProfileItem>, SwiftClientError> {
        Ok(self.client.user().refresh_user_profile(user_id).await?)
    }

    pub async fn bot_creator_id(
        &self,
        user_id: String,
    ) -> Result<Option<String>, SwiftClientError> {
        Ok(self.client.user().bot_creator_id(user_id).await?)
    }

    pub async fn users_by_identity_numbers(
        &self,
        identity_numbers: Vec<String>,
    ) -> Result<Vec<UserProfileItem>, SwiftClientError> {
        Ok(self
            .client
            .user()
            .users_by_identity_numbers(identity_numbers)
            .await?)
    }

    pub async fn selectable_users(&self) -> Result<Vec<UserProfileItem>, SwiftClientError> {
        Ok(self.client.user().selectable_users().await?)
    }

    pub async fn search_mao_user(
        &self,
        query: String,
    ) -> Result<Option<UserProfileItem>, SwiftClientError> {
        Ok(self.client.user().search_mao_user(query).await?)
    }

    pub async fn search_local_users(
        &self,
        query: String,
        category: String,
        limit: i64,
    ) -> Result<Vec<UserProfileItem>, SwiftClientError> {
        Ok(self
            .client
            .user()
            .search_local_users(query, category, limit)
            .await?)
    }

    pub async fn search_user(&self, query: String) -> Result<UserProfileItem, SwiftClientError> {
        Ok(self.client.user().search_user(query).await?)
    }

    pub async fn open_user_conversation(
        &self,
        user_id: String,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .open_user_conversation(user_id)
            .await?)
    }

    pub async fn create_group(
        &self,
        name: String,
        user_ids: Vec<String>,
    ) -> Result<String, SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .create_group(name, user_ids)
            .await?)
    }

    pub async fn create_circle(&self, name: String) -> Result<CircleItem, SwiftClientError> {
        Ok(self.client.conversation().create_circle(name).await?)
    }

    pub async fn update_circle(
        &self,
        circle_id: String,
        name: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .update_circle(circle_id, name)
            .await?)
    }

    pub async fn delete_circle(&self, circle_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.conversation().delete_circle(circle_id).await?)
    }

    pub async fn reorder_circles(&self, circle_ids: Vec<String>) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .reorder_circles(circle_ids)
            .await?)
    }

    pub async fn edit_circle_conversation(
        &self,
        circle_id: String,
        conversation_id: String,
        owner_id: String,
        category: String,
        add: bool,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .conversation()
            .edit_circle_conversation(
                circle_id,
                conversation_id,
                owner_id,
                category == "GROUP",
                add,
            )
            .await?)
    }

    pub async fn bot_home_uri(&self, app_id: String) -> Result<Option<String>, SwiftClientError> {
        Ok(self.client.user().bot_home_uri(app_id).await?)
    }

    pub async fn add_contact(
        &self,
        user_id: String,
        full_name: String,
    ) -> Result<(), SwiftClientError> {
        Ok(self.client.user().add_contact(user_id, full_name).await?)
    }

    pub async fn block_user(&self, user_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.user().block_user(user_id).await?)
    }

    pub async fn remove_contact(&self, user_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.user().remove_contact(user_id).await?)
    }

    pub async fn unblock_user(&self, user_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.user().unblock_user(user_id).await?)
    }

    pub async fn report_user(&self, user_id: String) -> Result<(), SwiftClientError> {
        Ok(self.client.user().report_user(user_id).await?)
    }

    pub async fn shutdown(&self) {
        self.client.shutdown().await;
    }

    pub async fn sign_out(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.sign_out().await?)
    }
}
