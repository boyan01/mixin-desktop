//! Account-owned pinned-message queries and commands.
use super::{model, MessageAccess};
use crate::db::mixin::{job::Job, message::Message, pin_message::PinMessageMinimal};
use anyhow::{anyhow, Result};
use chrono::Utc;
use log::warn;
use sdk::message_category::{MessageCategory as _, MESSAGE_PIN};
use sdk::{ConversationCategory, MessageStatus, PinMessagePayload};
use uuid::Uuid;

impl MessageAccess {
    pub async fn can_unpin_messages(&self, conversation_id: String) -> Result<bool> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .can_unpin_messages(&conversation_id, &self.account_id)
            .await?)
    }

    pub async fn pinned_audio_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .message_dao
            .list_pinned_audio_items_around(&conversation_id, &target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect()
    }

    pub async fn pinned_image_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::ImageMessageView>> {
        self.ensure_active()?;
        self.database
            .message_dao
            .list_pinned_image_items_around(&conversation_id, &target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.image_message_view(item, &conversation_id))
            .collect()
    }

    pub async fn pinned_messages_page(
        &self,
        conversation_id: String,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .message_dao
            .list_pinned_items_page(&conversation_id, offset, limit)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect()
    }

    pub async fn pinned_message_count(&self, conversation_id: String) -> Result<i64> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .pinned_message_count(&conversation_id)
            .await?)
    }

    pub async fn pinned_message_offset(
        &self,
        conversation_id: String,
        message_id: String,
    ) -> Result<Option<usize>> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .pinned_message_offset(&conversation_id, &message_id)
            .await?
            .map(|value| value as usize))
    }

    pub async fn unpin_all_messages(&self, conversation_id: String) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .message_dao
            .unpin_all_with_jobs(&conversation_id, &self.account_id)
            .await?;
        if let Err(error) = self.app_service.job.wake(sdk::PIN_MESSAGE) {
            warn!("Could not wake accepted unpin jobs: {error}");
        }
        self.notify_conversation_changed(&conversation_id);
        Ok(())
    }
    pub async fn pinned_messages(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::MessageListView>> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        self.database
            .message_dao
            .list_pinned_items(conversation_id)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()
    }

    pub async fn pinned_message_ids(&self, conversation_id: String) -> Result<Vec<String>> {
        self.ensure_active()?;
        Ok(self
            .database
            .pin_message_dao
            .message_ids(&conversation_id)
            .await?)
    }

    pub async fn pin_message_preview(
        &self,
        conversation_id: String,
    ) -> Result<Option<model::PinMessagePreviewItem>> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .pin_message_dao
            .latest_preview(conversation_id)
            .await?
            .map(|item| model::PinMessagePreviewItem {
                message_id: item.message_id,
                content: item.content.unwrap_or_default(),
                sender_name: item.sender_name.unwrap_or_default(),
            }))
    }

    pub async fn set_message_pinned(
        &self,
        conversation_id: String,
        message_id: String,
        pinned: bool,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?;
        let sent = matches!(
            message.status,
            MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
        );
        if message.conversation_id != conversation_id || !message.category.can_reply() || !sent {
            return Err(anyhow!("message can not be pinned: {message_id}"));
        }
        let participant = self
            .database
            .participant_dao
            .find_participant_by_id(conversation_id, &self.account_id)
            .await?;
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        if conversation.category == Some(ConversationCategory::Group)
            && participant.and_then(|item| item.role).is_none()
        {
            return Err(anyhow!("current user can not pin messages"));
        }
        let payload = if pinned {
            PinMessagePayload::Pin(vec![message_id.to_string()])
        } else {
            PinMessagePayload::Unpin(vec![message_id.to_string()])
        };
        let encoded = serde_json::to_string(&payload)?;
        let job = Job::create_send_pin_job(conversation_id, &encoded);
        let now = Utc::now();
        let pin_event = pinned
            .then(|| {
                Ok::<_, serde_json::Error>(Message {
                    message_id: Uuid::new_v4().to_string(),
                    conversation_id: conversation_id.to_string(),
                    user_id: self.account_id.clone(),
                    category: MESSAGE_PIN.to_string(),
                    content: Some(serde_json::to_string(&PinMessageMinimal {
                        category: message.category.clone(),
                        message_id: message.message_id.clone(),
                        content: if message.category.is_text() {
                            message.content.clone()
                        } else {
                            None
                        },
                    })?),
                    status: MessageStatus::Read,
                    created_at: now.naive_utc(),
                    quote_message_id: Some(message.message_id.clone()),
                    ..Message::default()
                })
            })
            .transpose()?;
        self.database
            .message_dao
            .set_message_pinned_with_job(
                conversation_id,
                message_id,
                pinned,
                now,
                pin_event.as_ref(),
                &job,
            )
            .await?;
        if pinned {
            if let Err(error) = self
                .settings
                .restore_pinned_preview(&self.account_id, conversation_id)
                .await
            {
                warn!("Could not restore pinned-message preview after accepted pin: {error}");
            }
        }
        if let Err(error) = self.app_service.job.wake(sdk::PIN_MESSAGE) {
            warn!("Could not wake accepted pin job: {error}");
        }
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }
}
