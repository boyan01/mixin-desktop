use std::sync::Arc;

use crate::subscription::{subscription, CancellableStream};
use futures::StreamExt as _;
use mixin_desktop_api::{
    AccountClient, CircleItem, ConversationChangeEvent, ConversationUnseenCount, MessageListView,
    NotificationEvent, UserProfileItem,
};

use crate::{
    error::SwiftClientError,
    model::{DeviceTransferEventItem, ParticipantAction, StickerAlbumSection, StickerLibrary},
};

#[derive(uniffi::Object)]
pub struct SwiftAccountHandle {
    client: Arc<AccountClient>,
}

subscription!(SwiftAccountHealthSubscription, String);

subscription!(SwiftConnectionSubscription, bool);

subscription!(SwiftCircleSubscription, Vec<CircleItem>);

subscription!(SwiftUnseenCountSubscription, Vec<ConversationUnseenCount>);

subscription!(SwiftUnseenMessageCountSubscription, i64);

#[derive(uniffi::Object)]
pub struct SwiftNotificationSubscription {
    inner: CancellableStream<Result<NotificationEvent, SwiftClientError>>,
}

subscription!(SwiftDeviceTransferSubscription, DeviceTransferEventItem);

subscription!(SwiftConversationSubscription, ConversationChangeEvent);

subscription!(SwiftMessageSubscription, u64);

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

    pub async fn shared_messages(
        &self,
        conversation_id: String,
        kind: String,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<MessageListView>, SwiftClientError> {
        Ok(self
            .client
            .message()
            .shared_messages(conversation_id, kind, offset as usize, limit as usize)
            .await?)
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
