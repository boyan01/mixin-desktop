use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use futures::StreamExt;
use mixin_desktop_core::core::device_transfer::{DeviceTransferCommand, DeviceTransferEvent};
use mixin_desktop_core::runtime::desktop::DesktopRuntime;
use mixin_desktop_core::runtime::model::{
    AccountProfile, ConversationChangeEvent, ConversationStorageUsage, ConversationUnseenCount,
    NotificationEvent, SnapshotDetailItem, StorageCategoryUsage,
};
use mixin_desktop_core::runtime::{
    AccountRuntime, AttachmentAccess, ConversationAccess, MessageAccess, StickerAccess, UserAccess,
};

use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(opaque)]
pub struct AccountHandle {
    runtime: Arc<AccountRuntime>,
    desktop: Arc<DesktopRuntime>,
}

impl AccountHandle {
    pub(super) fn new(runtime: Arc<AccountRuntime>, desktop: Arc<DesktopRuntime>) -> Self {
        Self { runtime, desktop }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn account_id(&self) -> String {
        self.runtime.account_id().to_string()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn profile(&self) -> AccountProfile {
        account_profile(&self.runtime.account())
    }

    pub async fn update_profile(
        &self,
        full_name: String,
        biography: String,
    ) -> Result<AccountProfile> {
        let account = self
            .runtime
            .update_account_profile(full_name, biography)
            .await?;
        Ok(account_profile(&account))
    }

    pub async fn refresh_profile(&self) -> Result<AccountProfile> {
        let account = self.runtime.refresh_account_profile().await?;
        Ok(account_profile(&account))
    }

    pub async fn profile_changes(&self, sink: StreamSink<AccountProfile>) -> Result<()> {
        let mut profile = self.runtime.subscribe_profile_changes();
        let mut shutdown = self.runtime.subscribe_shutdown();
        sink.add(account_profile(&profile.borrow().clone()))
            .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        loop {
            tokio::select! {
                changed = profile.changed() => {
                    if changed.is_err() {
                        return Ok(());
                    }
                    sink.add(account_profile(&profile.borrow_and_update().clone()))
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn storage_usage(&self) -> Result<Vec<ConversationStorageUsage>> {
        self.runtime.storage_usage().await
    }

    pub async fn snapshot_by_trace(&self, trace_id: String) -> Result<SnapshotDetailItem> {
        self.runtime.snapshot_by_trace(trace_id).await
    }

    pub async fn snapshot_by_id(&self, snapshot_id: String) -> Result<SnapshotDetailItem> {
        self.runtime.snapshot_by_id(snapshot_id).await
    }

    pub async fn safe_snapshot_by_id(&self, snapshot_id: String) -> Result<SnapshotDetailItem> {
        self.runtime.safe_snapshot_by_id(snapshot_id).await
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn media_directory(&self) -> Result<String> {
        Ok(self
            .runtime
            .media_directory()?
            .to_string_lossy()
            .into_owned())
    }

    pub async fn conversation_storage_usage(
        &self,
        conversation_id: String,
    ) -> Result<Vec<StorageCategoryUsage>> {
        self.runtime
            .conversation_storage_usage(conversation_id)
            .await
    }

    pub async fn clear_conversation_storage(
        &self,
        conversation_id: String,
        categories: Vec<String>,
    ) -> Result<()> {
        self.runtime
            .clear_conversation_storage(conversation_id, categories)
            .await
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn conversation(&self) -> ConversationAccess {
        self.runtime.conversation_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn message(&self) -> MessageAccess {
        self.runtime.message_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn attachment(&self) -> AttachmentAccess {
        self.runtime.attachment_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn download_progress(&self, message_id: String) -> f64 {
        self.runtime.attachment_progress(&message_id)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn sticker(&self) -> StickerAccess {
        self.runtime.sticker_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn user(&self) -> UserAccess {
        self.runtime.user_access()
    }

    pub async fn conversation_changes(
        &self,
        sink: StreamSink<ConversationChangeEvent>,
    ) -> Result<()> {
        forward_conversation_changes(&self.runtime, sink).await
    }

    pub async fn unseen_count_changes(
        &self,
        sink: StreamSink<Vec<ConversationUnseenCount>>,
    ) -> Result<()> {
        let mut changes = Box::pin(
            self.runtime
                .conversation_access()
                .subscribe_unseen_count_changes(),
        );
        while let Some(summary) = changes.next().await {
            if sink.add(summary).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn unseen_message_count_changes(&self, sink: StreamSink<i64>) -> Result<()> {
        let mut changes = Box::pin(
            self.runtime
                .conversation_access()
                .subscribe_unseen_message_count_changes(),
        );
        while let Some(count) = changes.next().await {
            if sink.add(count).is_err() {
                break;
            }
        }
        Ok(())
    }

    pub async fn message_changes(&self, sink: StreamSink<u64>) -> Result<()> {
        forward_message_changes(&self.runtime, sink).await
    }

    pub async fn desktop_notification_events(
        &self,
        sink: StreamSink<NotificationEvent>,
    ) -> Result<()> {
        let mut changes = self.runtime.subscribe_notification_changes();
        let mut shutdown = self.runtime.subscribe_shutdown();
        let mut created_at_micros = Utc::now().timestamp_micros();
        let mut row_id = self.runtime.latest_notification_row_id().await?;
        loop {
            loop {
                let batch = self
                    .runtime
                    .notification_event_batch(created_at_micros, row_id, 200)
                    .await?;
                created_at_micros = batch.next_created_at_micros;
                row_id = batch.next_row_id;
                for event in batch.events {
                    sink.add(event)
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                }
                if !batch.has_more {
                    break;
                }
            }
            tokio::select! {
                changed = changes.changed() => {
                    if changed.is_err() {
                        return Ok(());
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn device_transfer_events(
        &self,
        sink: StreamSink<DeviceTransferEvent>,
    ) -> Result<()> {
        let mut events = self.runtime.device_transfer().subscribe();
        let mut shutdown = self.runtime.subscribe_shutdown();
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => sink.add(event).map_err(|error| anyhow::anyhow!("{error:?}"))?,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
                },
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn device_transfer_command(&self, command: DeviceTransferCommand) -> Result<()> {
        self.runtime.device_transfer().command(command).await
    }

    pub async fn connection_status(&self, sink: StreamSink<bool>) -> Result<()> {
        let mut status = self.runtime.subscribe_connection_status();
        let mut shutdown = self.runtime.subscribe_shutdown();
        sink.add(*status.borrow())
            .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        loop {
            tokio::select! {
                changed = status.changed() => {
                    if changed.is_err() {
                        return Ok(());
                    }
                    sink.add(*status.borrow_and_update())
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn retry_connection(&self) {
        self.runtime.retry_connection();
    }

    pub async fn account_health(&self, sink: StreamSink<String>) -> Result<()> {
        let mut health = self.runtime.subscribe_account_health();
        let mut shutdown = self.runtime.subscribe_shutdown();
        sink.add(health.borrow().clone())
            .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        loop {
            tokio::select! {
                changed = health.changed() => {
                    if changed.is_err() {
                        return Ok(());
                    }
                    sink.add(health.borrow_and_update().clone())
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn refresh_account_health(&self) -> Result<()> {
        self.runtime.refresh_account_health().await
    }

    pub async fn shutdown(&self) {
        self.desktop.shutdown_account(&self.runtime).await;
    }

    pub async fn sign_out(&self) -> Result<()> {
        self.desktop.sign_out_account(&self.runtime).await
    }
}

fn account_profile(account: &sdk::Account) -> AccountProfile {
    AccountProfile {
        user_id: account.user_id.clone(),
        full_name: account.full_name.clone().unwrap_or_default(),
        avatar_url: account.avatar_url.clone().unwrap_or_default(),
        identity_number: account.identity_number.clone(),
        biography: account.biography.clone(),
        phone: account.phone.clone(),
        created_at: account.created_at.clone(),
        is_verified: account.is_verified,
        fiat_currency: account.fiat_currency.clone(),
        membership: account
            .membership
            .as_ref()
            .and_then(|membership| serde_json::to_string(membership).ok()),
    }
}

async fn forward_conversation_changes(
    runtime: &AccountRuntime,
    sink: StreamSink<ConversationChangeEvent>,
) -> Result<()> {
    use mixin_desktop_core::core::conversation_change::ConversationChange;

    let mut changes = runtime.subscribe_conversation_changes();
    let mut shutdown = runtime.subscribe_shutdown();
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
                if sink.add(event).is_err() {
                    break;
                }
            }
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn forward_message_changes(runtime: &AccountRuntime, sink: StreamSink<u64>) -> Result<()> {
    let mut changes = runtime.subscribe_message_changes();
    let mut shutdown = runtime.subscribe_shutdown();
    loop {
        if *shutdown.borrow() {
            break;
        }
        tokio::select! {
            result = changes.changed() => {
                if result.is_err() {
                    break;
                }
                if sink.add(*changes.borrow()).is_err() {
                    break;
                }
            }
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
    Ok(())
}
