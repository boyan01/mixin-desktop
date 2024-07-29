use std::default::Default;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::TimeDelta;
use log::{error, info};
use uuid::Uuid;

use sdk::blaze_message::{
    message_action, BlazeMessageData, MessageStatus, PlainJsonMessage, SnapshotMessage,
    SystemCircleMessage, SystemConversationMessage, SystemUserMessage,
    ACKNOWLEDGE_MESSAGE_RECEIPTS, RESEND_KEY, RESEND_MESSAGES,
};
use sdk::message_category::MessageCategory;
use sdk::{
    ack_message_status, message_category, AttachmentMessage, BlazeAckMessage, CircleConversation,
    ContactMessage, LiveMessage, PinMessagePayload, SafeSnapshotShot, StickerMessage,
    SystemCircleAction, SYSTEM_USER,
};

use crate::core::crypto::compose_message::ComposeMessageData;
use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::message::sender::{MessageSender, ProcessSignalKeyAction};
use crate::core::model::{AppService, AttachmentExtra};
use crate::core::util::generate_conversation_id;
use crate::db::mixin::conversation::ConversationStatus;
use crate::db::mixin::flood_message::FloodMessage;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::{AttachmentMessageUpdate, MediaStatus, Message};
use crate::db::mixin::participant::Participant;
use crate::db::mixin::pin_message::{PinMessage, PinMessageMinimal};
use crate::db::mixin::MixinDatabase;

pub struct ServiceDecryptMessage {
    database: Arc<MixinDatabase>,
    signal_protocol: Arc<SignalProtocol>,
    app_service: Arc<AppService>,
    sender: Arc<MessageSender>,
    user_id: String,
    identity_number: String,
}

impl ServiceDecryptMessage {
    pub fn new(
        database: Arc<MixinDatabase>,
        app_service: Arc<AppService>,
        signal_protocol: Arc<SignalProtocol>,
        sender: Arc<MessageSender>,
        user_id: String,
        identity_number: String,
    ) -> Self {
        Self {
            database,
            signal_protocol,
            app_service,
            sender,
            user_id,
            identity_number,
        }
    }

    pub async fn start(&self) {
        loop {
            let messages = self.database.flood_message_dao.flood_messages().await;
            if let Err(err) = messages {
                error!("failed to get messages: {:?}", err);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            let messages = messages.unwrap();
            info!("flood message count: {}", messages.len());
            for m in messages {
                if let Err(err) = self.process_message(&m).await {
                    error!("failed to process message: {:?}", err)
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    async fn process_message(&self, message: &FloodMessage) -> Result<()> {
        let data: BlazeMessageData = serde_json::from_slice(message.data.as_bytes())?;
        info!("process message: {} {}", data.message_id, data.category);
        if self
            .database
            .message_dao
            .is_message_exits(&message.message_id)
            .await?
        {
            self.update_remote_message_status(&message.message_id, MessageStatus::Delivered)
                .await?;
            self.database
                .flood_message_dao
                .delete_flood_message(&message.message_id)
                .await?;
            return Ok(());
        }

        let status = match self.parse_flood_message(&data).await {
            Err(err) => {
                error!("failed to handle flood message: {:?}.", err);
                self.handle_invalid_message(&data).await?
            }
            Ok(status) => status,
        };

        info!("message status: {:?}", status);

        Ok(())
    }

    async fn handle_invalid_message(&self, data: &BlazeMessageData) -> Result<MessageStatus> {
        if data.category == message_category::SIGNAL_KEY {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                content: Some(data.data.clone()),
                category: data.category.clone(),
                status: MessageStatus::Unknown,
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        }
        Ok(MessageStatus::Delivered)
    }

    async fn parse_flood_message(&self, data: &BlazeMessageData) -> Result<MessageStatus> {
        let category = &data.category;
        let mut status = MessageStatus::Delivered;
        self.app_service
            .conversation
            .sync_conversation(&data.conversation_id)
            .await?;
        let handled: Result<()> = if category.is_illegal_message_category() {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                created_at: data.created_at.naive_utc(),
                status: data.status,
                ..Message::default()
            };
            self.insert_message(&message, data).await
        } else if category.is_signal() {
            if data.category == message_category::SIGNAL_KEY {
                status = MessageStatus::Read;
                self.database
                    .message_history_dao
                    .insert(&data.message_id)
                    .await
                    .map_err(anyhow::Error::from)
            } else {
                self.process_signal_message(data).await
            }
        } else if category.is_plain() {
            self.process_plain_message(data).await
        } else if category.is_encrypted() {
            self.process_encrypted_message(data).await
        } else if category.is_system() {
            status = MessageStatus::Read;
            self.process_system_message(data).await
        } else if category.is_app_card() {
            self.process_app_card(data).await
        } else if category.is_app_button_group() {
            self.process_grouped_button(data).await
        } else if category.is_pin() {
            status = MessageStatus::Read;
            self.process_pin(data).await
        } else if category.is_recall() {
            status = MessageStatus::Read;
            self.process_recall(data).await
        } else {
            Ok(())
        };
        if let Err(err) = handled {
            error!("failed to process: {:?}", err);
            status = MessageStatus::Delivered;
            if category.is_location() {
                status = MessageStatus::Read;
            }
            self.handle_invalid_message(data).await?;
        }

        Ok(status)
    }
}

impl ServiceDecryptMessage {
    async fn update_remote_message_status(
        &self,
        message_id: &str,
        status: MessageStatus,
    ) -> Result<()> {
        if status != MessageStatus::Delivered && status != MessageStatus::Read {
            return Ok(());
        }
        self.app_service
            .job
            .add(&Job::create_ack_job(
                ACKNOWLEDGE_MESSAGE_RECEIPTS,
                message_id,
                status.into(),
                None,
            ))
            .await?;
        Ok(())
    }
}

impl ServiceDecryptMessage {
    async fn process_re_decrypted_message(
        &self,
        data: &BlazeMessageData,
        message_id: &str,
        plain_text: &str,
    ) -> Result<()> {
        if data.category == message_category::SIGNAL_TEXT {
            self.database
                .message_mention_dao
                .parse_and_save_mention_data(
                    message_id,
                    &data.conversation_id,
                    plain_text,
                    data.sender_id(),
                    None,
                    self.user_id.as_str(),
                    self.identity_number.as_str(),
                )
                .await?;
            self.database
                .message_dao
                .update_message_content_and_status(message_id, plain_text, data.status)
                .await?;
        } else if data.category == message_category::SIGNAL_POST
            || data.category == message_category::SIGNAL_LOCATION
        {
            self.database
                .message_dao
                .update_message_content_and_status(message_id, plain_text, data.status)
                .await?;
        } else if data.category.is_attachment() {
            let attachment: AttachmentMessage = serde_json::from_str(&decode(plain_text)?)?;
            let content = serde_json::to_string(&AttachmentExtra {
                attachment_id: attachment.attachment_id,
                message_id: data.message_id.clone(),
                shareable: attachment.shareable,
                created_at: None,
            })?;

            let message_update = AttachmentMessageUpdate {
                status: data.status,
                content,
                media_mine_type: attachment.mime_type,
                media_size: attachment.size,
                media_status: MediaStatus::Canceled,
                media_width: attachment.width,
                media_height: attachment.height,
                media_digest: attachment.digest,
                media_key: attachment.key,
                media_waveform: attachment.waveform,
                caption: attachment.caption,
                name: attachment.name,
                thumb_image: attachment.thumbnail,
                media_duration: attachment.duration.map(|d| d.to_string()),
            };
            self.database
                .message_dao
                .update_attachment_message(message_id, &message_update)
                .await?;

            // TODO(BIN): download attachment
        } else if data.category == message_category::SIGNAL_STICKER {
            let sticker_message: StickerMessage = serde_json::from_str(&decode(plain_text)?)?;
            let sticker = self
                .database
                .sticker_dao
                .find_sticker_by_id(&sticker_message.sticker_id)
                .await?;
            if sticker.is_none()
                || sticker.is_some_and(|s| {
                    s.album_id.is_none() || s.album_id.is_some_and(|a| a.is_empty())
                })
            {
                self.app_service
                    .job
                    .add(&Job::create_update_asset_job(&sticker_message.sticker_id))
                    .await?;
            }

            self.database
                .message_dao
                .update_sticker_message(message_id, sticker_message.sticker_id, data.status)
                .await?;
        } else if data.category == message_category::SIGNAL_CONTACT {
            let contact_message: ContactMessage = serde_json::from_str(&decode(plain_text)?)?;
            self.database
                .message_dao
                .update_contact_message(message_id, contact_message.user_id, data.status)
                .await?;
        } else if data.category == message_category::SIGNAL_LIVE {
            let live_message: LiveMessage = serde_json::from_str(&decode(plain_text)?)?;
            self.database
                .message_dao
                .update_live_message(
                    message_id,
                    live_message.width,
                    live_message.height,
                    &live_message.url,
                    &live_message.thumb_url,
                    data.status,
                )
                .await?;
        } else if data.category == message_category::SIGNAL_TRANSCRIPT {
            // TODO(BIN): handle transcript
        }

        self.database
            .message_dao
            .update_message_quote_if_need(&data.conversation_id, message_id)
            .await?;

        Ok(())
    }

    async fn process_signal_message(&self, data: &BlazeMessageData) -> Result<()> {
        let message_data = ComposeMessageData::decode(&data.data)
            .with_context(|| "failed to decode message data")?;
        let plain_text = self
            .signal_protocol
            .decrypt(
                &data.conversation_id,
                &data.user_id,
                message_data.key_type,
                message_data.cipher,
                &data.category.clone(),
                Some(&data.session_id),
            )
            .await
            .map_err(|e| anyhow!("failed to decrypt message: {e}"))?;
        if data.category != message_category::SIGNAL_KEY {
            let plain = std::str::from_utf8(&plain_text)?;
            if let Some(resend_message_id) = message_data.resend_message_id {
                self.process_re_decrypted_message(&data, &resend_message_id, plain)
                    .await?;
            } else {
                self.process_decrypt_success(data, plain).await?;
            }
        }
        Ok(())
    }

    async fn process_decrypt_success(
        &self,
        data: &BlazeMessageData,
        plain_text: &str,
    ) -> Result<()> {
        self.app_service
            .conversation
            .refresh_user(&[data.user_id.clone()], false)
            .await?;
        let quote_message = if let Some(quote_message_id) = data.quote_message_id.clone() {
            self.database
                .message_dao
                .find_quote_message_by_id(&quote_message_id)
                .await?
        } else {
            None
        };
        if data.category.is_text() {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                content: Some(plain_text.to_string()),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                quote_message_id: data.quote_message_id.clone(),
                quote_content: quote_message
                    .clone()
                    .map(|m| serde_json::to_string(&m).unwrap_or_default()),
                ..Message::default()
            };
            self.database
                .message_mention_dao
                .parse_and_save_mention_data(
                    &message.message_id,
                    &message.conversation_id,
                    message.content.as_deref(),
                    &data.user_id,
                    &quote_message,
                    self.user_id.as_str(),
                    self.identity_number.as_str(),
                )
                .await?;
            self.insert_message(&message, data).await?
        } else if data.category.is_attachment() {
            let attachment: AttachmentMessage = serde_json::from_str(plain_text)?;
            let content = serde_json::to_string(&AttachmentExtra {
                attachment_id: attachment.attachment_id,
                message_id: data.message_id.clone(),
                shareable: attachment.shareable,
                created_at: None,
            })?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                content: Some(content),
                name: attachment.name,
                media_mime_type: Some(attachment.mime_type),
                media_duration: attachment.duration.unwrap_or_default().to_string(),
                media_size: Some(attachment.size),
                media_width: attachment.width,
                media_height: attachment.height,
                thumb_image: attachment.thumbnail,
                media_key: attachment.key,
                media_digest: attachment.digest,
                status: data.status,
                created_at: data.created_at.naive_utc(),
                media_status: MediaStatus::Canceled,
                quote_message_id: data.quote_message_id.clone(),
                quote_content: quote_message
                    .clone()
                    .map(|m| serde_json::to_string(&m).unwrap_or_default()),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
            // TODO(BIN): download attachment
        } else if data.category.is_sticker() {
            let sticker_message: StickerMessage = serde_json::from_str(plain_text)?;
            let sticker = self
                .database
                .sticker_dao
                .find_sticker_by_id(&sticker_message.sticker_id)
                .await?;
            if sticker.is_none()
                || sticker.is_some_and(|s| {
                    s.album_id.is_none() || s.album_id.is_some_and(|a| a.is_empty())
                })
            {
                self.app_service
                    .job
                    .add(&Job::create_update_asset_job(&sticker_message.sticker_id))
                    .await?;
            }
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                content: Some(plain_text.to_string()),
                name: Some(sticker_message.name),
                sticker_id: Some(sticker_message.sticker_id),
                album_id: sticker_message.album_id,
                status: data.status,
                created_at: data.created_at.naive_utc(),
                quote_message_id: data.quote_message_id.clone(),
                quote_content: quote_message
                    .clone()
                    .map(|m| serde_json::to_string(&m).unwrap_or_default()),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        } else if data.category.is_contact() {
            let contact_message: ContactMessage = serde_json::from_str(plain_text)?;
            let users = self
                .app_service
                .conversation
                .refresh_user(&[contact_message.user_id], false)
                .await?;
            let user = users.first().ok_or(anyhow!("failed to find user"))?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                content: Some(plain_text.to_string()),
                shared_user_id: Some(user.user_id.clone()),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                quote_message_id: data.quote_message_id.clone(),
                quote_content: quote_message
                    .clone()
                    .map(|m| serde_json::to_string(&m).unwrap_or_default()),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        } else if data.category.is_live() {
            let live_message: LiveMessage = serde_json::from_str(plain_text)?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                content: Some(plain_text.to_string()),
                media_width: Some(live_message.width),
                media_height: Some(live_message.height),
                media_url: Some(live_message.url),
                thumb_url: Some(live_message.thumb_url),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        } else if data.category.is_location() {
            let location_message: sdk::LocationMessage = serde_json::from_str(plain_text)?;
            if location_message.latitude == 0.0 || location_message.longitude == 0.0 {
                return Err(anyhow!("invalid location message: {}", plain_text));
            }
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.user_id.clone(),
                category: data.category.clone(),
                content: Some(plain_text.to_string()),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        } else if data.category.is_transcript() {
            // TODO(BIN): process transcript
            return Err(anyhow!("transcript message: {}", plain_text));
        }
        Ok(())
    }

    async fn process_encrypted_message(&self, data: &BlazeMessageData) -> Result<()> {
        Ok(())
    }

    async fn process_plain_message(&self, data: &BlazeMessageData) -> Result<()> {
        let bytes = Base64::decode_vec(&data.data)?;
        let content = String::from_utf8_lossy(&bytes);
        if data.category == message_category::PLAIN_JSON {
            let plain_json_message: PlainJsonMessage = serde_json::from_str(&content)?;
            if plain_json_message.action == ACKNOWLEDGE_MESSAGE_RECEIPTS {
                if let Some(ack_messages) = plain_json_message.ack_messages {
                    self.mark_message_status(ack_messages).await?
                }
            } else if plain_json_message.action == RESEND_MESSAGES {
                self.process_resend_message(data, plain_json_message)
                    .await?
            } else if plain_json_message.action == RESEND_KEY
                && self
                    .signal_protocol
                    .protocol_store
                    .session_store
                    .contain_user_session(&data.user_id)
                    .await?
            {
                // TODO(BIN): resend session key
            }
            self.database
                .message_history_dao
                .insert(&data.message_id)
                .await?;
        } else if data.category == message_category::PLAIN_TEXT
            || data.category == message_category::PLAIN_IMAGE
            || data.category == message_category::PLAIN_VIDEO
            || data.category == message_category::PLAIN_DATA
            || data.category == message_category::PLAIN_AUDIO
            || data.category == message_category::PLAIN_CONTACT
            || data.category == message_category::PLAIN_STICKER
            || data.category == message_category::PLAIN_LIVE
            || data.category == message_category::PLAIN_POST
            || data.category == message_category::PLAIN_LOCATION
            || data.category == message_category::PLAIN_TRANSCRIPT
        {
            self.process_decrypt_success(data, &content).await?
        }
        Ok(())
    }
}

impl ServiceDecryptMessage {}

impl ServiceDecryptMessage {
    async fn process_resend_message(
        &self,
        data: &BlazeMessageData,
        plain_json_message: PlainJsonMessage,
    ) -> Result<()> {
        let messages = plain_json_message
            .messages
            .ok_or_else(|| anyhow!("no messages"))?;

        let p = self
            .database
            .participant_dao
            .find_participant_by_id(&data.conversation_id, &data.user_id)
            .await?
            .ok_or_else(|| anyhow!("no participant"))?;

        for message_id in messages {
            info!("resend message: {}", message_id);
            // TODO (BIN): resend message
        }
        Ok(())
    }

    async fn process_grouped_button(&self, data: &BlazeMessageData) -> Result<()> {
        let content = decode(&data.data)?;
        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.user_id.clone(),
            category: data.category.clone(),
            content: Some(content),
            status: data.status,
            created_at: data.created_at.naive_utc(),
            ..Message::default()
        };
        self.insert_message(&message, data).await?;
        Ok(())
    }

    async fn process_app_card(&self, data: &BlazeMessageData) -> Result<()> {
        self.app_service
            .conversation
            .refresh_user(&[data.user_id.clone()], false)
            .await?;
        let content = decode(&data.data)?;

        let app_card: sdk::AppCard = serde_json::from_str(&content)?;
        let app = self
            .database
            .app_dao
            .find_app_by_id(&app_card.app_id)
            .await?;
        if app.is_none() || app.is_some_and(|a| a.updated_at != app_card.updated_at) {
            self.app_service
                .conversation
                .refresh_user(&[data.user_id.clone()], true)
                .await?;
        }

        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.user_id.clone(),
            category: data.category.clone(),
            content: Some(content),
            status: data.status,
            created_at: data.created_at.naive_utc(),
            ..Message::default()
        };
        self.insert_message(&message, data).await?;
        Ok(())
    }

    async fn process_pin(&self, data: &BlazeMessageData) -> Result<()> {
        let payload: PinMessagePayload = serde_json::from_str(&data.data)?;
        match payload {
            PinMessagePayload::Pin(ids) => {
                for (i, mid) in ids.iter().enumerate() {
                    let message = self.database.message_dao.find_message_by_id(mid).await?;
                    let Some(message) = message else {
                        continue;
                    };

                    let pin_message_minimal = PinMessageMinimal {
                        category: message.category.clone(),
                        message_id: message.message_id.clone(),
                        content: if message.category == message_category::PLAIN_TEXT {
                            message.content.clone()
                        } else {
                            None
                        },
                    };
                    self.database
                        .pin_message_dao
                        .insert_pin_message(&PinMessage {
                            message_id: message.message_id.clone(),
                            conversation_id: message.conversation_id.clone(),
                            created_at: data.created_at,
                        })
                        .await?;
                    let message = Message {
                        message_id: if i == 0 {
                            data.message_id.clone()
                        } else {
                            Uuid::new_v4().to_string()
                        },
                        conversation_id: data.conversation_id.clone(),
                        quote_message_id: Some(message.message_id),
                        user_id: data.user_id.clone(),
                        status: MessageStatus::Read,
                        content: Some(serde_json::to_string(&pin_message_minimal)?),
                        created_at: data.created_at.naive_utc(),
                        category: message_category::MESSAGE_PIN.to_string(),
                        ..Message::default()
                    };
                    self.insert_message(&message, data).await?;
                }
            }
            PinMessagePayload::Unpin(message_ids) => {
                self.database
                    .pin_message_dao
                    .delete_pin_message(&message_ids)
                    .await?;
            }
        }
        self.database
            .message_history_dao
            .insert(&data.message_id)
            .await?;
        Ok(())
    }

    async fn process_recall(&self, data: &BlazeMessageData) -> Result<()> {
        Ok(())
    }
}

fn decode(data: &str) -> Result<String> {
    let decoded = Base64::decode_vec(data)?;
    Ok(String::from_utf8_lossy(&decoded).to_string())
}

impl ServiceDecryptMessage {
    async fn process_system_message(&self, data: &BlazeMessageData) -> Result<()> {
        let content = decode(&data.data)?;
        if data.category == message_category::SYSTEM_CONVERSATION {
            let message: SystemConversationMessage = serde_json::from_str(&content)?;
            self.process_system_conversation_message(data, message)
                .await?
        } else if data.category == message_category::SYSTEM_USER {
            let message: SystemUserMessage = serde_json::from_str(&content)?;
            self.process_system_user_message(message).await?
        } else if data.category == message_category::SYSTEM_CIRCLE {
            let message: SystemCircleMessage = serde_json::from_str(&content)?;
            self.process_system_circle_message(data, message).await?
        } else if data.category == message_category::SYSTEM_ACCOUNT_SNAPSHOT {
            let snapshot: SnapshotMessage = serde_json::from_str(&content)?;
            self.process_snapshot_message(data, snapshot).await?
        } else if data.category == message_category::SYSTEM_SAFE_SNAPSHOT {
            let snapshot: SafeSnapshotShot = serde_json::from_str(&content)?;
            self.process_safe_snapshot_message(data, snapshot).await?
        } else if data.category == message_category::SYSTEM_SAFE_INSCRIPTION {
            let snapshot: SafeSnapshotShot = serde_json::from_str(&content)?;
            self.process_safe_inscription_message(data, snapshot)
                .await?
        }
        Ok(())
    }

    async fn process_system_conversation_message(
        &self,
        data: &BlazeMessageData,
        message: SystemConversationMessage,
    ) -> Result<()> {
        if message.action != message_action::UPDATE {
            self.app_service
                .conversation
                .sync_conversation(&data.conversation_id)
                .await?
        }
        let user_id: &str = message.user_id.as_ref().unwrap_or(data.sender_id());
        if user_id == SYSTEM_USER {
            self.database
                .user_dao
                .insert_system_user_if_not_exist()
                .await?
        }

        if message.action == message_action::JOIN || message.action == message_action::ADD {
            self.database
                .participant_dao
                .insert_participant(&Participant {
                    conversation_id: data.conversation_id.clone(),
                    user_id: data.sender_id().to_string(),
                    role: message.role,
                    created_at: data.created_at,
                })
                .await?;
            if message.participant_id == self.user_id {
                self.app_service
                    .conversation
                    .refresh_conversation(&data.conversation_id)
                    .await?;
            } else if self
                .signal_protocol
                .protocol_store
                .sender_key_store
                .exists_sender_key(&data.conversation_id, &message.participant_id)
                .await?
            {
                self.sender
                    .send_process_signal_key(
                        data,
                        ProcessSignalKeyAction::AddParticipant(&message.participant_id),
                    )
                    .await?;
                self.app_service
                    .conversation
                    .refresh_user(&[message.participant_id.clone()], false)
                    .await?;
            } else {
                let user_ids = &[message.participant_id.clone()];
                self.app_service
                    .conversation
                    .refresh_session(&data.conversation_id, user_ids)
                    .await?;
                self.app_service
                    .conversation
                    .refresh_user(user_ids, false)
                    .await?;
            }
        } else if message.action == message_action::REMOVE || message.action == message_action::EXIT
        {
            if message.participant_id == self.user_id {
                self.database
                    .conversation_dao
                    .update_status(&data.conversation_id, ConversationStatus::QUIT)
                    .await?;
            }
            self.app_service
                .conversation
                .refresh_user(&[message.participant_id.clone()], false)
                .await?;
            self.sender
                .send_process_signal_key(
                    data,
                    ProcessSignalKeyAction::RemoveParticipant(&message.participant_id),
                )
                .await?;
        } else if message.action == message_action::UPDATE {
            if !message.participant_id.is_empty() {
                self.app_service
                    .conversation
                    .refresh_user(&[message.participant_id.clone()], true)
                    .await?;
            } else {
                self.app_service
                    .conversation
                    .refresh_conversation(&data.conversation_id)
                    .await?;
            }
        } else if message.action == message_action::ROLE {
            self.database
                .participant_dao
                .update_participant_role(
                    &data.conversation_id,
                    &message.participant_id,
                    &message.role,
                )
                .await?;
            if message.participant_id != self.user_id || message.role.is_none() {
                return Ok(());
            }
        } else if message.action == message_action::EXPIRE {
            self.database
                .conversation_dao
                .update_expire_in(&data.conversation_id, message.expire_in.unwrap_or_default())
                .await?;
        }

        let m = Message {
            message_id: data.message_id.clone(),
            user_id: data.user_id.clone(),
            conversation_id: data.conversation_id.clone(),
            category: data.category.clone(),
            content: if message.action == message_action::EXPIRE {
                Some(message.expire_in.unwrap_or_default().to_string())
            } else {
                Some("".to_string())
            },
            created_at: data.created_at.naive_utc(),
            status: data.status,
            action: Some(message.action.clone()),
            participant_id: Some(message.participant_id.clone()),
            ..Message::default()
        };
        self.insert_message(&m, data).await?;
        Ok(())
    }

    async fn process_system_user_message(&self, m: SystemUserMessage) -> Result<()> {
        if m.action == message_action::UPDATE {
            self.app_service
                .conversation
                .refresh_user(&[m.user_id.clone()], true)
                .await?;
        }
        Ok(())
    }

    async fn process_system_circle_message(
        &self,
        data: &BlazeMessageData,
        message: SystemCircleMessage,
    ) -> Result<()> {
        if message.action == SystemCircleAction::Create
            || message.action == SystemCircleAction::Update
        {
            self.app_service
                .circle
                .refresh_circle(&message.circle_id)
                .await?;
        } else if message.action == SystemCircleAction::Add {
            self.app_service
                .circle
                .sync_circle(&message.circle_id)
                .await?;
            if let Some(user_id) = message.user_id.as_ref() {
                self.app_service
                    .conversation
                    .refresh_user(&[user_id.to_string()], false)
                    .await?;
            }
            let conversation_id = message.conversation_id.unwrap_or(
                generate_conversation_id(
                    &self.user_id,
                    message
                        .user_id
                        .as_ref()
                        .ok_or(anyhow!("system_circle_message: user id is empty"))?,
                )
                .to_string(),
            );
            self.database
                .circle_conversation_dao
                .insert(&[CircleConversation {
                    conversation_id,
                    circle_id: message.circle_id.clone(),
                    user_id: message.user_id.clone(),
                    created_at: data.created_at,
                    pin_time: None,
                }])
                .await?;
        } else if message.action == SystemCircleAction::Remove {
            let conversation_id = message.conversation_id.unwrap_or(
                generate_conversation_id(
                    &self.user_id,
                    message
                        .user_id
                        .as_ref()
                        .ok_or(anyhow!("system_circle_message: user id is empty"))?,
                )
                .to_string(),
            );
            self.database
                .circle_conversation_dao
                .delete(&message.circle_id, &conversation_id)
                .await?;
        } else if message.action == SystemCircleAction::Delete {
            self.database.circle_dao.delete(&message.circle_id).await?;
            self.database
                .circle_conversation_dao
                .delete_by_circle(&message.circle_id)
                .await?;
        }
        Ok(())
    }

    async fn process_snapshot_message(
        &self,
        data: &BlazeMessageData,
        snapshot: SnapshotMessage,
    ) -> Result<()> {
        self.database.snapshot_dao.insert(&snapshot).await?;
        self.app_service
            .job
            .add(&Job::create_update_asset_job(&snapshot.asset_id))
            .await?;

        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.user_id.clone(),
            category: data.category.clone(),
            content: Some("".to_string()),
            snapshot_id: Some(snapshot.snapshot_id),
            status: data.status,
            created_at: data.created_at.naive_utc(),
            ..Message::default()
        };
        self.insert_message(&message, data).await?;
        Ok(())
    }

    async fn process_safe_snapshot_message(
        &self,
        data: &BlazeMessageData,
        snapshot: SafeSnapshotShot,
    ) -> Result<()> {
        if !snapshot.transaction_hash.is_empty() {
            self.database
                .safe_snapshot_dao
                .delete_pending_snapshot_by_hash(&snapshot.transaction_hash)
                .await?;
        }
        self.database.safe_snapshot_dao.insert(&snapshot).await?;

        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.sender_id().clone(),
            category: data.category.clone(),
            content: Some("".to_string()),
            snapshot_id: Some(snapshot.snapshot_id),
            status: data.status,
            created_at: data.created_at.naive_utc(),
            action: Some(snapshot.type_field),
            ..Message::default()
        };
        self.insert_message(&message, data).await?;

        Ok(())
    }

    async fn process_safe_inscription_message(
        &self,
        data: &BlazeMessageData,
        snapshot: SafeSnapshotShot,
    ) -> Result<()> {
        self.database.safe_snapshot_dao.insert(&snapshot).await?;
        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.sender_id().clone(),
            category: data.category.clone(),
            content: snapshot.inscription_hash,
            snapshot_id: Some(snapshot.snapshot_id),
            status: data.status,
            created_at: data.created_at.naive_utc(),
            action: Some(snapshot.type_field),
            ..Message::default()
        };
        self.insert_message(&message, data).await?;
        self.app_service
            .job
            .add(&Job::create_sync_inscription_message_job(&data.message_id))
            .await?;
        Ok(())
    }
}

impl ServiceDecryptMessage {
    async fn mark_message_status(&self, blaze_messages: Vec<BlazeAckMessage>) -> Result<()> {
        let mut messages_mention_read = Vec::new();
        let mut message_read_with_expires = Vec::new();
        let mut message_read = Vec::new();

        for m in blaze_messages {
            if m.status == ack_message_status::MENTION_READ {
                messages_mention_read.push(m.message_id);
            } else if m.status == ack_message_status::READ {
                let expired_at = m.expire_at.unwrap_or(0);
                if expired_at > 0 {
                    message_read_with_expires.push((m.message_id, expired_at));
                } else {
                    message_read.push(m.message_id);
                }
            }
        }

        self.database
            .message_mention_dao
            .mark_mention_read(&messages_mention_read)
            .await?;

        self.app_service
            .message
            .mark_message_read(&message_read, true)
            .await?;

        let message = message_read_with_expires
            .iter()
            .map(|(message_id, _)| message_id.to_string())
            .collect::<Vec<_>>();
        self.app_service
            .message
            .mark_message_read(&message, false)
            .await?;
        self.database
            .expired_message_dao
            .update_message_expired_at(&message_read_with_expires)
            .await?;

        Ok(())
    }

    async fn insert_message(&self, message: &Message, data: &BlazeMessageData) -> Result<()> {
        info!(
            "insert message: {:?} {:?}",
            message.message_id, message.content
        );
        self.database.message_dao.insert_message(message).await?;
        // TODO(BIN): insert fts
        let expire_in = data.expire_in.unwrap_or(0);
        if expire_in > 0 && message.user_id == self.user_id {
            let expire_at = data.created_at + TimeDelta::seconds(expire_in);
            self.database
                .expired_message_dao
                .update_message_expired_at(&[(
                    data.message_id.clone(),
                    expire_at.timestamp_millis() / 1000,
                )])
                .await?;
        }
        Ok(())
    }
}
