use std::sync::Arc;

use futures::StreamExt;
use mixin_desktop_api::{
    AccountClient, AccountProfile, AttachmentAccess, CircleItem, ConversationAccess,
    ConversationChangeEvent, ConversationStorageUsage, ConversationUnseenCount, MessageAccess,
    NotificationEvent, SnapshotDetailItem, StickerAccess, StorageCategoryUsage, UserAccess,
};

use crate::api::device_transfer::{DeviceTransferCommand, DeviceTransferEvent};
use crate::{frb_generated::StreamSink, CoreError, Result};

#[flutter_rust_bridge::frb(opaque)]
pub struct AccountHandle {
    client: Arc<AccountClient>,
}

impl AccountHandle {
    pub(super) fn new(client: AccountClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn account_id(&self) -> String {
        self.client.account_id()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn profile(&self) -> AccountProfile {
        self.client.profile()
    }

    pub async fn update_profile(
        &self,
        full_name: String,
        biography: String,
    ) -> Result<AccountProfile, CoreError> {
        Ok(self.client.update_profile(full_name, biography).await?)
    }

    pub async fn refresh_profile(&self) -> Result<AccountProfile, CoreError> {
        Ok(self.client.refresh_profile().await?)
    }
}

impl AccountHandle {
    pub async fn profile_changes(&self, sink: StreamSink<AccountProfile>) -> Result<(), CoreError> {
        let changes = self.client.profile_changes();
        futures::pin_mut!(changes);
        while let Some(profile) = changes.next().await {
            if sink.add(profile).is_err() {
                break;
            }
        }
        Ok(())
    }
}

impl AccountHandle {
    pub async fn storage_usage(&self) -> Result<Vec<ConversationStorageUsage>, CoreError> {
        Ok(self.client.storage_usage().await?)
    }

    pub async fn snapshot_by_trace(
        &self,
        trace_id: String,
    ) -> Result<SnapshotDetailItem, CoreError> {
        Ok(self.client.snapshot_by_trace(trace_id).await?)
    }

    pub async fn snapshot_by_id(
        &self,
        snapshot_id: String,
    ) -> Result<SnapshotDetailItem, CoreError> {
        Ok(self.client.snapshot_by_id(snapshot_id).await?)
    }

    pub async fn safe_snapshot_by_id(
        &self,
        snapshot_id: String,
    ) -> Result<SnapshotDetailItem, CoreError> {
        Ok(self.client.safe_snapshot_by_id(snapshot_id).await?)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn media_directory(&self) -> Result<String, CoreError> {
        Ok(self.client.media_directory()?)
    }

    pub async fn conversation_storage_usage(
        &self,
        conversation_id: String,
    ) -> Result<Vec<StorageCategoryUsage>, CoreError> {
        Ok(self
            .client
            .conversation_storage_usage(conversation_id)
            .await?)
    }

    pub async fn clear_conversation_storage(
        &self,
        conversation_id: String,
        categories: Vec<String>,
    ) -> Result<(), CoreError> {
        Ok(self
            .client
            .clear_conversation_storage(conversation_id, categories)
            .await?)
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn conversation(&self) -> ConversationAccess {
        self.client.conversation()
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn message(&self) -> MessageAccess {
        self.client.message()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn attachment(&self) -> AttachmentAccess {
        self.client.attachment()
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn attachment_progress(&self, message_id: String) -> f64 {
        self.client.attachment_progress(message_id)
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn sticker(&self) -> StickerAccess {
        self.client.sticker()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn user(&self) -> UserAccess {
        self.client.user()
    }

    pub async fn conversation_changes(
        &self,
        sink: StreamSink<ConversationChangeEvent>,
    ) -> Result<(), CoreError> {
        let changes = self.client.conversation_changes();
        futures::pin_mut!(changes);
        while let Some(event) = changes.next().await {
            if sink.add(event).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn circle_changes(&self, sink: StreamSink<Vec<CircleItem>>) -> Result<(), CoreError> {
        let mut changes = Box::pin(self.client.circle_changes());
        while let Some(circles) = changes.next().await {
            if sink.add(circles).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn unseen_count_changes(
        &self,
        sink: StreamSink<Vec<ConversationUnseenCount>>,
    ) -> Result<(), CoreError> {
        let mut changes = Box::pin(self.client.unseen_count_changes());
        while let Some(summary) = changes.next().await {
            if sink.add(summary).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn unseen_message_count_changes(
        &self,
        sink: StreamSink<i64>,
    ) -> Result<(), CoreError> {
        let mut changes = Box::pin(self.client.unseen_message_count_changes());
        while let Some(count) = changes.next().await {
            if sink.add(count).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn message_changes(&self, sink: StreamSink<u64>) -> Result<(), CoreError> {
        let changes = self.client.message_changes();
        futures::pin_mut!(changes);
        while let Some(revision) = changes.next().await {
            if sink.add(revision).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn desktop_notification_events(
        &self,
        sink: StreamSink<NotificationEvent>,
    ) -> Result<(), CoreError> {
        let events = self.client.notification_events();
        futures::pin_mut!(events);
        while let Some(event) = events.next().await {
            if sink.add(event?).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn device_transfer_events(
        &self,
        sink: StreamSink<DeviceTransferEvent>,
    ) -> Result<(), CoreError> {
        let events = self.client.device_transfer_events();
        futures::pin_mut!(events);
        while let Some(event) = events.next().await {
            if sink.add(event.into()).is_err() {
                break;
            }
        }
        Ok(())
    }
}

impl AccountHandle {
    pub async fn device_transfer_command(
        &self,
        command: DeviceTransferCommand,
    ) -> Result<(), CoreError> {
        Ok(self.client.device_transfer_command(command.into()).await?)
    }
}

impl AccountHandle {
    pub async fn connection_status(&self, sink: StreamSink<bool>) -> Result<(), CoreError> {
        let status = self.client.connection_status();
        futures::pin_mut!(status);
        while let Some(connected) = status.next().await {
            if sink.add(connected).is_err() {
                break;
            }
        }
        Ok(())
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn retry_connection(&self) {
        self.client.retry_connection();
    }
}

impl AccountHandle {
    pub async fn account_health(&self, sink: StreamSink<String>) -> Result<(), CoreError> {
        let health = self.client.account_health();
        futures::pin_mut!(health);
        while let Some(value) = health.next().await {
            if sink.add(value).is_err() {
                break;
            }
        }
        Ok(())
    }
}

impl AccountHandle {
    pub async fn refresh_account_health(&self) -> Result<(), CoreError> {
        Ok(self.client.refresh_account_health().await?)
    }

    pub async fn shutdown(&self) {
        self.client.shutdown().await;
    }

    pub async fn sign_out(&self) -> Result<(), CoreError> {
        Ok(self.client.sign_out().await?)
    }
}
