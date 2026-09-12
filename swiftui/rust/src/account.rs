use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::{Stream, StreamExt as _};
use mixin_desktop_api::{
    AccountClient, CircleItem, ConversationChangeEvent, ConversationUnseenCount, NotificationEvent,
    UserProfileItem,
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

    pub fn device_transfer_events(&self) -> SwiftDeviceTransferSubscription {
        SwiftDeviceTransferSubscription {
            inner: CancellableStream::new(self.client.device_transfer_events().map(Into::into)),
        }
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

    pub async fn user_profile(
        &self,
        user_id: String,
    ) -> Result<Option<UserProfileItem>, SwiftClientError> {
        Ok(self.client.user().user_profile(Some(user_id), None).await?)
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
}

include!("generated/account.rs");
