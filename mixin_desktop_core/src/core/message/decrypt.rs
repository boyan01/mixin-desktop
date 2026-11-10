use crate::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Encoding};
use log::{error, info, warn};
use tokio::sync::{broadcast, mpsc, watch};
use uuid::Uuid;

use sdk::blaze_message::{
    message_action, system_session_action, BlazeMessageData, MessageStatus, PlainJsonMessage,
    SnapshotMessage, SystemCircleMessage, SystemConversationMessage, SystemSessionMessage,
    SystemUserMessage, ACKNOWLEDGE_MESSAGE_RECEIPTS, NO_KEY, RESEND_KEY, RESEND_MESSAGES,
};
use sdk::message_category::MessageCategory;
use sdk::{
    ack_message_status, message_category, AttachmentMessage, BlazeAckMessage, CircleConversation,
    ContactMessage, LiveMessage, PinMessagePayload, SafeSnapshotShot, StickerMessage,
    SystemCircleAction, SYSTEM_USER,
};

use crate::core::conversation_change::ConversationChangeNotifier;
use crate::core::crypto::compose_message::ComposeMessageData;
use crate::core::crypto::encrypted_protocol;
use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::device_transfer::{DeviceTransferControlEvent, DEVICE_TRANSFER_ACTION};
use crate::core::message::blaze::PendingMessageStatusStore;
use crate::core::message::sender::{MessageSender, ProcessSignalKeyAction};
use crate::core::model::{AppService, AttachmentExtra};
use crate::db::app::{Auth, SettingDao};
use crate::db::mixin::conversation::ConversationStatus;
use crate::db::mixin::flood_message::FloodMessage;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::{AttachmentMessageUpdate, MediaStatus, Message};
use crate::db::mixin::message_fts::message_fts_content;
use crate::db::mixin::participant::Participant;
use crate::db::mixin::pin_message::{PinMessage, PinMessageMinimal};
use crate::db::mixin::transcript_message::TranscriptMessage;
use crate::db::mixin::MixinDatabase;
use crate::db::signal::ratchet_sender_key::ratchet_sender_key_status;
use sdk::generate_conversation_id;

#[derive(Clone)]
pub struct ServiceDecryptMessage {
    database: Arc<MixinDatabase>,
    signal_protocol: Arc<SignalProtocol>,
    app_service: Arc<AppService>,
    sender: Arc<MessageSender>,
    user_id: String,
    identity_number: String,
    private_key: Vec<u8>,
    session_id: String,
    pending_message_statuses: PendingMessageStatusStore,
    conversation_changes: Option<ConversationChangeNotifier>,
    notification_changes: Option<watch::Sender<u64>>,
    device_transfer_controls: Option<broadcast::Sender<DeviceTransferControlEvent>>,
    attachment_transfer_requests: Option<mpsc::UnboundedSender<AttachmentTransferRequest>>,
    settings: Option<SettingDao>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AttachmentTransferRequest {
    pub(crate) message_id: String,
    pub(crate) category: String,
    pub(crate) transcript_id: Option<String>,
    pub(crate) cancel: bool,
}

struct PreparedTranscript {
    summary: String,
    fts_content: String,
    media_size: i64,
    media_status: MediaStatus,
    attachments: Vec<TranscriptMessage>,
}

const FLOOD_MESSAGE_RETRY_DELAY: Duration = Duration::from_secs(1);
const FLOOD_MESSAGE_IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(42);

impl ServiceDecryptMessage {
    pub fn new(
        database: Arc<MixinDatabase>,
        app_service: Arc<AppService>,
        signal_protocol: Arc<SignalProtocol>,
        sender: Arc<MessageSender>,
        pending_message_statuses: PendingMessageStatusStore,
        auth: &Auth,
    ) -> Self {
        Self {
            database,
            signal_protocol,
            app_service,
            sender,
            user_id: auth.account.user_id.clone(),
            identity_number: auth.account.identity_number.clone(),
            private_key: auth.private_key.clone(),
            session_id: auth.account.session_id.clone(),
            pending_message_statuses,
            conversation_changes: None,
            notification_changes: None,
            device_transfer_controls: None,
            attachment_transfer_requests: None,
            settings: None,
        }
    }

    pub fn with_conversation_changes(mut self, sender: ConversationChangeNotifier) -> Self {
        self.conversation_changes = Some(sender);
        self
    }

    pub(crate) fn with_settings(mut self, settings: SettingDao) -> Self {
        self.settings = Some(settings);
        self
    }

    pub fn with_notification_changes(mut self, sender: watch::Sender<u64>) -> Self {
        self.notification_changes = Some(sender);
        self
    }

    pub fn with_device_transfer_controls(
        mut self,
        sender: broadcast::Sender<DeviceTransferControlEvent>,
    ) -> Self {
        self.device_transfer_controls = Some(sender);
        self
    }

    pub(crate) fn with_attachment_transfer_requests(
        mut self,
        sender: mpsc::UnboundedSender<AttachmentTransferRequest>,
    ) -> Self {
        self.attachment_transfer_requests = Some(sender);
        self
    }

    fn request_attachment_download(
        &self,
        message_id: &str,
        category: &str,
        transcript_id: Option<&str>,
    ) {
        let Some(sender) = &self.attachment_transfer_requests else {
            return;
        };
        if sender
            .send(AttachmentTransferRequest {
                message_id: message_id.to_string(),
                category: category.to_string(),
                transcript_id: transcript_id.map(str::to_string),
                cancel: false,
            })
            .is_err()
        {
            warn!("attachment transfer request receiver is closed");
        }
    }

    fn request_attachment_cancel(&self, message_id: &str, transcript_id: Option<&str>) {
        let Some(sender) = &self.attachment_transfer_requests else {
            return;
        };
        if sender
            .send(AttachmentTransferRequest {
                message_id: message_id.to_string(),
                category: String::new(),
                transcript_id: transcript_id.map(str::to_string),
                cancel: true,
            })
            .is_err()
        {
            warn!("attachment transfer request receiver is closed");
        }
    }

    pub async fn start(&self) {
        loop {
            let started_at = Instant::now();
            let mut processed = 0;
            let mut should_retry = false;

            loop {
                let messages = match self.database.flood_message_dao.flood_messages().await {
                    Ok(messages) => messages,
                    Err(err) => {
                        error!("failed to get messages: {:?}", err);
                        should_retry = true;
                        break;
                    }
                };

                if messages.is_empty() {
                    break;
                }

                for message in messages {
                    match self.process_message(&message).await {
                        Ok(conversation_id) => {
                            processed += 1;
                            if let Some(sender) = &self.conversation_changes {
                                sender.notify(conversation_id);
                            }
                            if let Some(sender) = &self.notification_changes {
                                sender.send_modify(|revision| {
                                    *revision = revision.wrapping_add(1);
                                });
                            }
                        }
                        Err(err) => {
                            error!("failed to process message: {:?}", err);
                            should_retry = true;
                        }
                    }
                }

                if should_retry {
                    break;
                }
            }

            if processed > 0 {
                info!(
                    "processed {} flood messages in {} ms",
                    processed,
                    started_at.elapsed().as_millis()
                );
            }

            let delay = if should_retry {
                FLOOD_MESSAGE_RETRY_DELAY
            } else {
                FLOOD_MESSAGE_IDLE_CHECK_INTERVAL
            };
            tokio::select! {
                _ = self.database.flood_message_dao.wait_for_message() => {}
                _ = crate::time::sleep(delay) => {}
            }
        }
    }

    async fn process_message(&self, message: &FloodMessage) -> Result<String> {
        let data: BlazeMessageData = serde_json::from_slice(message.data.as_bytes())?;
        let conversation_id = data.conversation_id.clone();
        info!("process message: {} {}", data.message_id, data.category);
        if self
            .database
            .message_dao
            .is_message_exits(&message.message_id)
            .await?
            || self
                .database
                .message_history_dao
                .exists(&message.message_id)
                .await?
        {
            self.update_remote_message_status(&message.message_id, MessageStatus::Delivered)
                .await?;
            self.database
                .flood_message_dao
                .delete_flood_message(&message.message_id)
                .await?;
            return Ok(conversation_id);
        }

        let status = match self.parse_flood_message(&data).await {
            Err(err) => {
                error!("failed to handle flood message: {:?}.", err);
                self.handle_invalid_message(&data).await?
            }
            Ok(status) => status,
        };

        self.update_remote_message_status(&message.message_id, status)
            .await?;
        self.database
            .flood_message_dao
            .delete_flood_message(&message.message_id)
            .await?;
        Ok(conversation_id)
    }

    async fn handle_invalid_message(&self, data: &BlazeMessageData) -> Result<MessageStatus> {
        if data.category != message_category::SIGNAL_KEY {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                content: Some(data.data.clone()),
                category: data.category.clone(),
                status: MessageStatus::Unknown,
                created_at: data.created_at.naive_utc(),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        }
        Ok(MessageStatus::Delivered)
    }

    /// A sender on a session we have not seen must receive our sender key on
    /// the next send.
    async fn check_session(&self, data: &BlazeMessageData) -> Result<()> {
        if data.conversation_id == SYSTEM_USER
            || data.conversation_id == self.user_id
            || data.user_id == SYSTEM_USER
        {
            return Ok(());
        }
        self.database
            .participant_session_dao
            .insert_if_absent(&data.conversation_id, &data.user_id, &data.session_id)
            .await?;
        Ok(())
    }

    async fn parse_flood_message(&self, data: &BlazeMessageData) -> Result<MessageStatus> {
        let category = &data.category;
        let mut status = MessageStatus::Delivered;
        self.app_service
            .conversation
            .sync_conversation(&data.conversation_id)
            .await?;
        self.check_session(data).await?;
        let handled: Result<()> = if category.is_illegal_message_category() {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
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
                    .map_err(anyhow::Error::from)?;
            }
            self.process_signal_message(data).await
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

#[cfg(test)]
mod transcript_tests {
    use super::{decode_transcript_material, transcript_attachment_id};

    #[test]
    fn parses_transcript_attachment_extra_or_raw_id() {
        assert_eq!(
            transcript_attachment_id(
                r#"{"attachment_id":"attachment-id","message_id":"message-id"}"#
            )
            .unwrap(),
            "attachment-id"
        );
        assert_eq!(
            transcript_attachment_id("legacy-attachment-id").unwrap(),
            "legacy-attachment-id"
        );
        assert_eq!(decode_transcript_material(Some("")).unwrap(), None);
        assert_eq!(
            decode_transcript_material(Some("AQI=")).unwrap(),
            Some(vec![1, 2])
        );
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
    async fn prepare_transcript(
        &self,
        message_id: &str,
        content: &str,
    ) -> Result<PreparedTranscript> {
        let mut transcripts: Vec<TranscriptMessage> = serde_json::from_str(content)?;
        transcripts.retain(|transcript| transcript.transcript_id == message_id);
        if transcripts.is_empty() {
            bail!("transcript does not contain its root message");
        }

        let mut user_ids = Vec::new();
        let mut seen_user_ids = HashSet::new();
        for transcript in &mut transcripts {
            if transcript.category.is_attachment() {
                transcript.media_status = Some(MediaStatus::Canceled);
            }
            if let Some(user_id) = transcript.user_id.as_deref() {
                if !user_id.is_empty() && seen_user_ids.insert(user_id.to_string()) {
                    user_ids.push(user_id.to_string());
                }
            }
            if transcript.category.is_contact() {
                if let Some(user_id) = transcript.shared_user_id.as_deref() {
                    if !user_id.is_empty() && seen_user_ids.insert(user_id.to_string()) {
                        user_ids.push(user_id.to_string());
                    }
                }
            }
            if transcript.category.is_sticker() {
                if let Some(sticker_id) = transcript.sticker_id.as_deref() {
                    if !sticker_id.is_empty()
                        && self
                            .database
                            .sticker_dao
                            .find_sticker_by_id(sticker_id)
                            .await?
                            .is_none()
                    {
                        self.app_service
                            .job
                            .add(&Job::create_update_sticker_job(sticker_id))
                            .await?;
                    }
                }
            }
        }

        let users = self
            .app_service
            .conversation
            .cached_users_or_enqueue(&user_ids, false)
            .await?;
        let user_names = users
            .into_iter()
            .map(|user| (user.user_id, user.full_name))
            .collect::<HashMap<_, _>>();

        self.database
            .transcript_message_dao
            .insert_all(&transcripts)
            .await?;
        transcripts.sort_by_key(|transcript| transcript.created_at);

        let summary = transcripts
            .iter()
            .map(|transcript| {
                serde_json::json!({
                    "name": transcript.user_full_name.as_deref().unwrap_or_default(),
                    "category": transcript.category,
                    "content": transcript.content,
                })
            })
            .collect::<Vec<_>>();
        let summary = serde_json::to_string(&summary)?;

        let attachments = transcripts
            .iter()
            .filter(|transcript| transcript.category.is_attachment())
            .cloned()
            .collect::<Vec<_>>();
        let media_size = attachments.iter().try_fold(0_i64, |total, transcript| {
            total
                .checked_add(transcript.media_size.unwrap_or_default())
                .ok_or_else(|| anyhow!("transcript media size overflow"))
        })?;
        let media_status = if attachments.is_empty() || media_size == 0 {
            MediaStatus::Done
        } else {
            MediaStatus::Canceled
        };

        let fts_content = transcripts
            .iter()
            .filter_map(|transcript| {
                if transcript.category.is_text() || transcript.category.is_post() {
                    transcript.content.clone()
                } else if transcript.category.is_data() {
                    transcript.media_name.clone()
                } else if transcript.category.is_contact() {
                    transcript
                        .shared_user_id
                        .as_ref()
                        .and_then(|user_id| user_names.get(user_id))
                        .cloned()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        Ok(PreparedTranscript {
            summary,
            fts_content,
            media_size,
            media_status,
            attachments,
        })
    }

    fn request_transcript_attachment_downloads(
        &self,
        transcript_id: &str,
        attachments: &[TranscriptMessage],
    ) {
        for transcript in attachments {
            self.request_attachment_download(
                &transcript.message_id,
                &transcript.category,
                Some(transcript_id),
            );
        }
    }

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
                message_id: message_id.to_string(),
                shareable: attachment.shareable,
                created_at: attachment.created_at,
            })?;

            let message_update = AttachmentMessageUpdate {
                status: data.status,
                content,
                media_mime_type: attachment.mime_type,
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
            self.request_attachment_download(message_id, &data.category, None);
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
                    .add(&Job::create_update_sticker_job(&sticker_message.sticker_id))
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
            let transcript = self.prepare_transcript(message_id, plain_text).await?;
            self.database
                .message_dao
                .update_transcript_message(
                    message_id,
                    &transcript.summary,
                    transcript.media_size,
                    transcript.media_status,
                    data.status,
                )
                .await?;
            self.database
                .message_dao
                .update_message_quote_if_need(&data.conversation_id, message_id)
                .await?;
            self.database
                .message_fts_dao
                .upsert(message_id, &data.conversation_id, &transcript.fts_content)
                .await?;
            self.request_transcript_attachment_downloads(message_id, &transcript.attachments);
            return Ok(());
        }

        self.database
            .message_dao
            .update_message_quote_if_need(&data.conversation_id, message_id)
            .await?;
        if let Some(message) = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
        {
            if message.category.is_fts_message() {
                if let Some(content) = message.content.as_deref() {
                    self.database
                        .message_fts_dao
                        .upsert(message_id, &data.conversation_id, content)
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn process_signal_message(&self, data: &BlazeMessageData) -> Result<()> {
        let message_data = ComposeMessageData::decode(&data.data)
            .with_context(|| "failed to decode message data")?;
        let plain_text = self
            .signal_protocol
            .decrypt(
                &data.conversation_id,
                data.sender_id(),
                message_data.key_type,
                message_data.cipher,
                &data.category.clone(),
                Some(&data.session_id),
            )
            .await
            .with_context(|| format!("failed to decrypt message: {}", data.message_id));

        let device_id = SignalProtocol::device_id(Some(&data.session_id))?;
        let address = format!("{}:{}", data.sender_id(), device_id);

        let plain_text = match plain_text {
            Ok(text) => text,
            Err(err) => {
                error!("failed to decrypt message:{} {:?}", data.message_id, err);
                self.sender
                    .refresh_signal_key(&data.conversation_id)
                    .await?;

                if data.category == message_category::SIGNAL_KEY {
                    self.signal_protocol
                        .signal_database
                        .ratchet_sender_key_dao
                        .delete(&data.conversation_id, &address)
                        .await?;
                } else {
                    self.insert_failed_message(data).await?;
                    let status = self
                        .signal_protocol
                        .signal_database
                        .ratchet_sender_key_dao
                        .find_status(&data.conversation_id, &address)
                        .await?;
                    if status.is_none() {
                        self.sender
                            .request_resend_key(
                                &data.conversation_id,
                                data.sender_id(),
                                &data.message_id,
                                &data.session_id,
                            )
                            .await?;
                    }
                }
                return Ok(());
            }
        };

        if data.category != message_category::SIGNAL_KEY {
            let plain = std::str::from_utf8(&plain_text)?;
            if let Some(resend_message_id) = message_data.resend_message_id {
                self.process_re_decrypted_message(data, &resend_message_id, plain)
                    .await?;
                self.database
                    .message_history_dao
                    .insert(&data.message_id)
                    .await?;
            } else {
                self.process_decrypt_success(data, plain).await?;
            }
        }

        let status = self
            .signal_protocol
            .signal_database
            .ratchet_sender_key_dao
            .find_status(&data.conversation_id, &address)
            .await?;
        if status == Some(ratchet_sender_key_status::REQUESTING.to_string()) {
            self.sender
                .request_resend_message(&data.conversation_id, data.sender_id(), &data.session_id)
                .await?;
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
            .cached_users_or_enqueue(std::slice::from_ref(data.sender_id()), false)
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
            let plain = decode_content(data, plain_text)?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(plain),
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
                    data.sender_id(),
                    &quote_message,
                    self.user_id.as_str(),
                    self.identity_number.as_str(),
                )
                .await?;
            self.insert_message(&message, data).await?
        } else if data.category.is_attachment() {
            let plain = decode_content(data, plain_text)?;
            let attachment: AttachmentMessage = serde_json::from_str(&plain)?;
            let content = serde_json::to_string(&AttachmentExtra {
                attachment_id: attachment.attachment_id,
                message_id: data.message_id.clone(),
                shareable: attachment.shareable,
                created_at: attachment.created_at,
            })?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(content),
                name: attachment.name,
                media_mime_type: Some(attachment.mime_type),
                media_duration: attachment
                    .duration
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                media_size: Some(attachment.size),
                media_width: attachment.width,
                media_height: attachment.height,
                thumb_image: attachment.thumbnail,
                media_key: attachment.key,
                media_digest: attachment.digest,
                media_waveform: attachment.waveform.as_deref().map(Base64::encode_string),
                caption: attachment.caption,
                status: data.status,
                created_at: data.created_at.naive_utc(),
                media_status: MediaStatus::Canceled,
                quote_message_id: data.quote_message_id.clone(),
                quote_content: quote_message
                    .clone()
                    .map(|m| serde_json::to_string(&m).unwrap_or_default()),
                ..Message::default()
            };
            self.insert_message(&message, data).await?;
            self.request_attachment_download(&message.message_id, &message.category, None);
        } else if data.category.is_sticker() {
            let plain = decode_content(data, plain_text)?;
            let sticker_message: StickerMessage = serde_json::from_str(&plain)?;
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
                    .add(&Job::create_update_sticker_job(&sticker_message.sticker_id))
                    .await?;
            }
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(plain),
                name: sticker_message.name,
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
            let plain = decode_content(data, plain_text)?;
            let contact_message: ContactMessage = serde_json::from_str(&plain)?;
            let users = self
                .app_service
                .conversation
                .cached_users_or_enqueue(std::slice::from_ref(&contact_message.user_id), false)
                .await?;
            let user = users.first();
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(plain),
                name: user.map(|user| user.full_name.clone()),
                shared_user_id: Some(contact_message.user_id),
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
            let plain = decode_content(data, plain_text)?;
            let live_message: LiveMessage = serde_json::from_str(&plain)?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(plain),
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
            let plain = decode_content(data, plain_text)?;
            let location_message: sdk::LocationMessage = serde_json::from_str(&plain)?;
            if location_message.latitude == 0.0 || location_message.longitude == 0.0 {
                return Err(anyhow!("invalid location message: {}", plain));
            }
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(plain),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        } else if data.category.is_post() {
            let plain = decode_content(data, plain_text)?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(plain),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                quote_message_id: data.quote_message_id.clone(),
                quote_content: quote_message
                    .clone()
                    .map(|message| serde_json::to_string(&message).unwrap_or_default()),
                ..Message::default()
            };
            self.insert_message(&message, data).await?
        } else if data.category.is_transcript() {
            let plain = decode_content(data, plain_text)?;
            let transcript = self.prepare_transcript(&data.message_id, &plain).await?;
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(transcript.summary.clone()),
                media_size: Some(transcript.media_size),
                status: data.status,
                created_at: data.created_at.naive_utc(),
                media_status: transcript.media_status,
                ..Message::default()
            };
            self.insert_message(&message, data).await?;
            self.database
                .message_fts_dao
                .upsert(
                    &data.message_id,
                    &data.conversation_id,
                    &transcript.fts_content,
                )
                .await?;
            self.request_transcript_attachment_downloads(&data.message_id, &transcript.attachments);
        }
        Ok(())
    }

    async fn process_encrypted_message(&self, data: &BlazeMessageData) -> Result<()> {
        let cipher_text = Base64::decode_vec(&data.data)?;
        let plain_text =
            encrypted_protocol::decrypt_message(&self.private_key, &self.session_id, &cipher_text)?;
        let plain_text = String::from_utf8_lossy(&plain_text);
        self.process_decrypt_success(data, &plain_text).await?;
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
                self.sender
                    .send_process_signal_key(data, ProcessSignalKeyAction::ResendKey)
                    .await?;
            } else if plain_json_message.action == NO_KEY {
                let device_id = SignalProtocol::device_id(Some(&data.session_id))?;
                self.signal_protocol
                    .signal_database
                    .ratchet_sender_key_dao
                    .delete(
                        &data.conversation_id,
                        &format!("{}:{}", data.user_id, device_id),
                    )
                    .await?;
            } else if plain_json_message.action == DEVICE_TRANSFER_ACTION {
                if let (Some(sender), Some(content)) =
                    (&self.device_transfer_controls, plain_json_message.content)
                {
                    let _ = sender.send(DeviceTransferControlEvent { content });
                }
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
            self.process_decrypt_success(data, &data.data).await?
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
        let Some(messages) = plain_json_message.messages else {
            return Ok(());
        };
        let Some(participant) = self
            .database
            .participant_dao
            .find_participant_by_id(&data.conversation_id, &data.user_id)
            .await?
        else {
            return Ok(());
        };
        let resends = &self.database.participant_session_dao;

        for message_id in messages {
            if resends
                .has_resend_request(&data.user_id, &data.session_id, &message_id)
                .await?
            {
                continue;
            }
            info!("resend message: {}", message_id);
            let message = self
                .database
                .message_dao
                .find_message_by_id(&message_id)
                .await?
                .filter(|message| {
                    message.user_id == self.user_id
                        && message.category != message_category::MESSAGE_RECALL
                });
            let Some(message) = message else {
                resends
                    .insert_resend_request(&data.user_id, &data.session_id, &message_id, 0)
                    .await?;
                continue;
            };
            if !message.category.is_signal()
                || participant.created_at.naive_utc() > message.created_at
            {
                continue;
            }
            self.app_service
                .job
                .add(&Job::create_sending_job(
                    &message_id,
                    &data.conversation_id,
                    Some(&data.user_id),
                    Some(&data.session_id),
                    true,
                    false,
                    data.expire_in.unwrap_or_default(),
                ))
                .await?;
            resends
                .insert_resend_request(&data.user_id, &data.session_id, &message_id, 1)
                .await?;
        }
        Ok(())
    }

    async fn process_grouped_button(&self, data: &BlazeMessageData) -> Result<()> {
        let content = decode(&data.data)?;
        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.sender_id().clone(),
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
            .cached_users_or_enqueue(std::slice::from_ref(&data.user_id), false)
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
                .cached_users_or_enqueue(std::slice::from_ref(&app_card.app_id), true)
                .await?;
        }

        let message = Message {
            message_id: data.message_id.clone(),
            conversation_id: data.conversation_id.clone(),
            user_id: data.sender_id().clone(),
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
        let payload: PinMessagePayload = serde_json::from_str(&decode(&data.data)?)?;
        match payload {
            PinMessagePayload::Pin(ids) => {
                if let Some(settings) = &self.settings {
                    settings
                        .restore_pinned_preview(&self.user_id, &data.conversation_id)
                        .await?;
                }
                for (i, mid) in ids.iter().enumerate() {
                    let message = self.database.message_dao.find_message_by_id(mid).await?;
                    let Some(message) = message else {
                        continue;
                    };

                    let pin_message_minimal = PinMessageMinimal {
                        category: message.category.clone(),
                        message_id: message.message_id.clone(),
                        content: if message.category.is_text() {
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
                        user_id: data.sender_id().clone(),
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
        let recall: sdk::RecallMessage = serde_json::from_str(&decode(&data.data)?)?;
        let recalled = self
            .database
            .message_dao
            .find_message_by_id(&recall.message_id)
            .await?;
        let mut media_urls = if recalled
            .as_ref()
            .is_some_and(|message| message.category.is_transcript())
        {
            for transcript in self
                .database
                .transcript_message_dao
                .find_by_transcript_id(&recall.message_id)
                .await?
                .into_iter()
                .filter(|message| message.category.is_attachment())
            {
                self.request_attachment_cancel(&transcript.message_id, Some(&recall.message_id));
            }
            self.database
                .transcript_message_dao
                .media_urls_by_transcript_id(&recall.message_id)
                .await?
        } else {
            Vec::new()
        };
        if recalled
            .as_ref()
            .is_some_and(|message| message.category.is_attachment())
        {
            self.request_attachment_cancel(&recall.message_id, None);
            if let Some(path) = recalled
                .as_ref()
                .and_then(|message| message.media_url.clone())
            {
                media_urls.push(path);
            }
        }
        self.database
            .message_dao
            .recall_message(&data.conversation_id, &recall.message_id)
            .await?;
        self.database
            .message_history_dao
            .insert(&data.message_id)
            .await?;
        media_urls.sort_unstable();
        media_urls.dedup();
        for path in media_urls
            .into_iter()
            .filter(|path| std::path::Path::new(path).is_absolute())
        {
            if let Err(err) = crate::fs::remove_file(&path).await {
                if err.kind() != std::io::ErrorKind::NotFound {
                    warn!("failed to remove recalled attachment {path}: {err}");
                }
            }
        }
        Ok(())
    }
}

fn decode(data: &str) -> Result<String> {
    let decoded = Base64::decode_vec(data)?;
    Ok(String::from_utf8_lossy(&decoded).to_string())
}

fn decode_content(data: &BlazeMessageData, plain_text: &str) -> Result<String> {
    let signal_content_is_plain = data.category.is_text()
        || data.category.is_post()
        || data.category.is_location()
        || data.category.is_transcript();
    if data.category.is_encrypted() || (data.category.is_signal() && signal_content_is_plain) {
        Ok(plain_text.to_string())
    } else {
        decode(plain_text)
    }
}

pub(crate) fn transcript_attachment_id(content: &str) -> Result<String> {
    if let Some(attachment_id) = serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|value| {
            value
                .get("attachment_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
    {
        if !attachment_id.trim().is_empty() {
            return Ok(attachment_id);
        }
    }

    let attachment_id = Base64::decode_vec(content)
        .ok()
        .and_then(|decoded| serde_json::from_slice::<AttachmentMessage>(&decoded).ok())
        .map(|attachment| attachment.attachment_id)
        .unwrap_or_else(|| content.to_string());
    if attachment_id.trim().is_empty() {
        bail!("transcript attachment id is empty");
    }
    Ok(attachment_id)
}

pub(crate) fn transcript_attachment_message(
    conversation_id: &str,
    transcript: &TranscriptMessage,
) -> Result<Message> {
    Ok(Message {
        message_id: transcript.message_id.clone(),
        conversation_id: conversation_id.to_string(),
        user_id: transcript.user_id.clone().unwrap_or_default(),
        category: transcript.category.clone(),
        content: transcript.content.clone(),
        media_url: transcript.media_url.clone(),
        media_mime_type: transcript.media_mime_type.clone(),
        media_size: transcript.media_size,
        media_duration: transcript.media_duration.clone().unwrap_or_default(),
        media_width: transcript.media_width,
        media_height: transcript.media_height,
        thumb_image: transcript.thumb_image.clone(),
        media_key: decode_transcript_material(transcript.media_key.as_deref())?,
        media_digest: decode_transcript_material(transcript.media_digest.as_deref())?,
        media_status: MediaStatus::Canceled,
        created_at: transcript.created_at.naive_utc(),
        name: transcript.media_name.clone(),
        media_waveform: transcript.media_waveform.clone(),
        thumb_url: transcript.thumb_url.clone(),
        caption: transcript.caption.clone(),
        ..Message::default()
    })
}

fn decode_transcript_material(value: Option<&str>) -> Result<Option<Vec<u8>>> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(Base64::decode_vec)
        .transpose()
        .map_err(Into::into)
}

impl ServiceDecryptMessage {
    /// Another session of a user was linked or removed: existing Signal
    /// sessions with that user are stale and sender keys must be redistributed.
    async fn process_system_session_message(&self, message: SystemSessionMessage) -> Result<()> {
        if message.session_id == self.session_id {
            return Ok(());
        }
        let sessions = &self.database.participant_session_dao;
        let signal_sessions = &self.signal_protocol.signal_database.session_dao;
        if message.action == system_session_action::PROVISION {
            signal_sessions.delete_sessions(&message.user_id).await?;
            sessions
                .provision(
                    &message.user_id,
                    &message.session_id,
                    message.public_key.as_deref(),
                )
                .await?;
        } else if message.action == system_session_action::DESTROY
            // The mobile clients likewise ignore a session they do not track.
            && sessions
                .has_user_session(&message.user_id, &message.session_id)
                .await?
        {
            signal_sessions.delete_sessions(&message.user_id).await?;
            sessions
                .delete_user_session(&message.user_id, &message.session_id)
                .await?;
        }
        Ok(())
    }

    async fn process_system_message(&self, data: &BlazeMessageData) -> Result<()> {
        let content = decode(&data.data)?;
        if data.category == message_category::SYSTEM_CONVERSATION {
            let message: SystemConversationMessage = serde_json::from_str(&content)?;
            self.process_system_conversation_message(data, message)
                .await?
        } else if data.category == message_category::SYSTEM_USER {
            let message: SystemUserMessage = serde_json::from_str(&content)?;
            self.process_system_user_message(message).await?
        } else if data.category == message_category::SYSTEM_SESSION {
            let message: SystemSessionMessage = serde_json::from_str(&content)?;
            self.process_system_session_message(message).await?
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
        let user_id = message
            .user_id
            .clone()
            .unwrap_or_else(|| data.sender_id().clone());
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
                    user_id: message.participant_id.clone(),
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
                .exists_sender_key(&data.conversation_id, &self.user_id)
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
                    .cached_users_or_enqueue(std::slice::from_ref(&message.participant_id), false)
                    .await?;
            } else {
                let user_ids = std::slice::from_ref(&message.participant_id);
                self.app_service
                    .conversation
                    .refresh_session(&data.conversation_id, user_ids)
                    .await?;
                self.app_service
                    .conversation
                    .cached_users_or_enqueue(user_ids, false)
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
                .cached_users_or_enqueue(std::slice::from_ref(&message.participant_id), false)
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
                    .cached_users_or_enqueue(std::slice::from_ref(&message.participant_id), true)
                    .await?;
            } else {
                self.app_service
                    .conversation
                    .refresh_conversation(&data.conversation_id)
                    .await?;
            }
            return Ok(());
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
            user_id,
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
                .cached_users_or_enqueue(std::slice::from_ref(&m.user_id), true)
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
            let app_service = self.app_service.clone();
            let circle_id = message.circle_id.clone();
            crate::task::spawn(async move {
                if let Err(error) = app_service.circle.refresh_circle(&circle_id).await {
                    warn!("failed to refresh circle {circle_id}: {error}");
                }
            });
        } else if message.action == SystemCircleAction::Add {
            if !self.database.circle_dao.exists(&message.circle_id).await? {
                let app_service = self.app_service.clone();
                let circle_id = message.circle_id.clone();
                crate::task::spawn(async move {
                    if let Err(error) = app_service.circle.refresh_circle(&circle_id).await {
                        warn!("failed to refresh circle {circle_id}: {error}");
                    }
                });
            }
            if let Some(user_id) = message.user_id.as_ref() {
                self.app_service
                    .conversation
                    .cached_users_or_enqueue(std::slice::from_ref(user_id), false)
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
            user_id: data.sender_id().clone(),
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
        mut snapshot: SafeSnapshotShot,
    ) -> Result<()> {
        let asset_id = snapshot.asset_id.clone();
        if let Some(deposit_hash) = snapshot
            .deposit_hash
            .as_deref()
            .filter(|hash| !hash.is_empty())
        {
            self.database
                .safe_snapshot_dao
                .delete_pending_snapshot_by_hash(deposit_hash)
                .await?;
            snapshot.deposit = Some(sdk::SafeDeposit {
                deposit_hash: deposit_hash.to_string(),
                sender: String::new(),
            });
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
        self.app_service
            .job
            .add(&Job::create_update_token_job(&asset_id))
            .await?;

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
        if !message_read_with_expires.is_empty() {
            self.app_service.expired_message.wake();
        }

        Ok(())
    }

    async fn insert_message(&self, message: &Message, data: &BlazeMessageData) -> Result<()> {
        info!(
            "insert message: {:?} {:?}",
            message.message_id, message.content
        );
        self.pending_message_statuses
            .insert_message(&self.database, message)
            .await?;
        self.database
            .conversation_dao
            .update_for_message(message, &self.user_id)
            .await?;
        if let Some(content) = message_fts_content(message) {
            self.database
                .message_fts_dao
                .upsert(&message.message_id, &message.conversation_id, &content)
                .await?;
        }
        let expire_in = data.expire_in.unwrap_or(0);
        if expire_in > 0 {
            let expire_at = (message.user_id == self.user_id)
                .then_some(data.created_at.timestamp() + expire_in);
            self.database
                .expired_message_dao
                .insert(&message.message_id, expire_in, expire_at)
                .await?;
            self.app_service.expired_message.wake();
        }
        Ok(())
    }

    async fn insert_failed_message(&self, data: &BlazeMessageData) -> Result<()> {
        if data.category.is_signal() && data.category != message_category::SIGNAL_KEY {
            let message = Message {
                message_id: data.message_id.clone(),
                conversation_id: data.conversation_id.clone(),
                user_id: data.sender_id().clone(),
                category: data.category.clone(),
                content: Some(data.data.clone()),
                status: MessageStatus::Failed,
                created_at: data.created_at.naive_utc(),
                ..Message::default()
            };
            self.insert_message(&message, data).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn decrypt_service_fixture() -> (
        tempfile::TempDir,
        Arc<MixinDatabase>,
        Arc<crate::db::SignalDatabase>,
        ServiceDecryptMessage,
    ) {
        use crate::core::attachment::AttachmentService;
        use crate::core::message::blaze::Blaze;
        use crate::core::model::{conversation::ConversationService, signal::SignalService};
        use crate::db::SignalDatabase;

        let directory = tempfile::tempdir().unwrap();
        let database = Arc::new(
            MixinDatabase::connect_at(directory.path().join("mixin.db"))
                .await
                .unwrap(),
        );
        let signal_database = Arc::new(
            SignalDatabase::connect_at(directory.path().join("signal.db"))
                .await
                .unwrap(),
        );
        // Invalid signing material rejects accidental API calls before any network I/O.
        let client = Arc::new(sdk::Client::new(sdk::Credential::KeyStore(sdk::KeyStore {
            app_id: "test".into(),
            session_id: "test".into(),
            server_public_key: String::new(),
            session_private_key: "invalid".into(),
            scp: String::new(),
        })));
        let mut account = serde_json::json!({});
        for key in [
            "user_id",
            "biography",
            "code_id",
            "code_url",
            "created_at",
            "device_status",
            "fiat_currency",
            "accept_search_source",
            "accept_conversation_source",
            "receive_message_source",
            "identity_number",
            "mute_until",
            "phone",
            "pin_token",
            "pin_token_base64",
            "salt_base64",
            "session_id",
            "spend_public_key",
            "tip_key_base64",
        ] {
            account[key] = "".into();
        }
        for key in [
            "has_emergency_contact",
            "has_pin",
            "has_safe",
            "is_deactivated",
            "is_scam",
            "is_verified",
        ] {
            account[key] = false.into();
        }
        for key in [
            "tip_counter",
            "transfer_confirmation_threshold",
            "transfer_notification_threshold",
        ] {
            account[key] = 0.into();
        }
        account["user_id"] = "me".into();
        account["identity_number"] = "1".into();
        let auth = Auth {
            user_id: "me".into(),
            private_key: vec![],
            primary_session_id: None,
            account: serde_json::from_value(account).unwrap(),
        };
        let conversation = ConversationService::new(database.clone(), client.clone(), "me".into());
        let protocol = Arc::new(SignalProtocol::new(signal_database.clone(), "me".into()));
        let blaze = Arc::new(Blaze::new(
            database.clone(),
            client.clone(),
            sdk::Credential::None,
            "me".into(),
            None,
        ));
        let sender = Arc::new(MessageSender::new(
            blaze.clone(),
            conversation,
            database.clone(),
            "me".into(),
            "".into(),
            protocol.clone(),
            SignalService::new(protocol.clone(), signal_database.clone(), client.clone()),
        ));
        let attachment = Arc::new(AttachmentService::new(
            client.clone(),
            reqwest::Client::new(),
            directory.path(),
        ));
        let app = Arc::new(AppService::new(
            database.clone(),
            client,
            &auth,
            sender.clone(),
            attachment,
            None,
        ));
        let service = ServiceDecryptMessage::new(
            database.clone(),
            app,
            protocol,
            sender,
            blaze.pending_message_statuses(),
            &auth,
        );
        let now = chrono::Utc::now();
        database
            .conversation_dao
            .insert(&crate::db::mixin::conversation::Conversation {
                conversation_id: "conversation".into(),
                owner_id: Some("missing-sender".into()),
                category: Some(sdk::ConversationCategory::Contact),
                name: String::new(),
                icon_url: String::new(),
                announcement: String::new(),
                code_url: String::new(),
                created_at: now,
                status: ConversationStatus::SUCCESS,
                mute_until: now,
                expire_in: 0,
            })
            .await
            .unwrap();
        (directory, database, signal_database, service)
    }

    const PEER_SESSION: &str = "10000000-0000-4000-8000-000000000002";

    fn blaze_data(category: &str, data: &str) -> BlazeMessageData {
        let now = chrono::Utc::now();
        serde_json::from_value(serde_json::json!({
            "conversation_id": "conversation", "user_id": "peer", "message_id": "event",
            "category": category, "data": data, "status": "SENT", "created_at": now,
            "updated_at": now, "source": "CREATE_MESSAGE", "session_id": PEER_SESSION
        }))
        .unwrap()
    }

    fn plain_json(action: &str) -> BlazeMessageData {
        let payload = serde_json::json!({ "action": action }).to_string();
        blaze_data("PLAIN_JSON", &Base64::encode_string(payload.as_bytes()))
    }

    #[tokio::test]
    async fn no_key_from_peer_clears_only_that_sessions_pending_key_request() {
        use crate::db::signal::ratchet_sender_key::RatchetSenderKey;
        let (_directory, _database, signal_database, service) = decrypt_service_fixture().await;
        let device = SignalProtocol::device_id(Some(PEER_SESSION)).unwrap();
        let peer = format!("peer:{device}");
        let dao = &signal_database.ratchet_sender_key_dao;
        for sender in [peer.as_str(), "other:1"] {
            dao.insert_sender_key(&RatchetSenderKey {
                group_id: "conversation".into(),
                sender_id: sender.into(),
                status: ratchet_sender_key_status::REQUESTING.into(),
                message_id: None,
                created_at: String::new(),
            })
            .await
            .unwrap();
        }

        service
            .process_plain_message(&plain_json(NO_KEY))
            .await
            .unwrap();

        assert_eq!(dao.find_status("conversation", &peer).await.unwrap(), None);
        assert!(dao
            .find_status("conversation", "other:1")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn incoming_message_records_new_session_and_keeps_known_delivery_state() {
        let (_directory, database, _signal, service) = decrypt_service_fixture().await;
        let dao = &database.participant_session_dao;
        dao.insert_session("conversation", "known", PEER_SESSION, 1)
            .await
            .unwrap();
        let mut known = plain_json("IGNORED");
        known.user_id = "known".into();
        let mut system = plain_json("IGNORED");
        system.user_id = SYSTEM_USER.into();

        for data in [plain_json("IGNORED"), known, system] {
            service.parse_flood_message(&data).await.unwrap();
        }

        let mut sessions = dao
            .get_participant_sessions("conversation")
            .await
            .unwrap()
            .into_iter()
            .map(|session| (session.user_id, session.sent_to_server))
            .collect::<Vec<_>>();
        sessions.sort();
        assert_eq!(
            sessions,
            [("known".to_owned(), Some(1)), ("peer".to_owned(), None)]
        );
    }

    fn system_session(action: &str, session: &str) -> BlazeMessageData {
        let payload = serde_json::json!({
            "action": action, "user_id": "me", "session_id": session, "public_key": "key"
        })
        .to_string();
        blaze_data("SYSTEM_SESSION", &Base64::encode_string(payload.as_bytes()))
    }

    #[tokio::test]
    async fn system_session_provision_targets_usable_conversations_and_drops_signal_sessions() {
        let (_directory, database, signal, service) = decrypt_service_fixture().await;
        let pool = &database.user_dao.0;
        for (user, app) in [
            ("friend", None),
            ("encrypted-bot", Some("encrypted-bot")),
            ("plain-bot", Some("plain-bot")),
        ] {
            crate::db::query(
                "INSERT INTO users (user_id, identity_number, full_name, avatar_url, phone, \
                 is_verified, created_at, mute_until, has_pin, biography, is_scam, code_url, \
                 code_id, is_deactivated, app_id) \
                 VALUES (?1, ?1, '', '', '', 0, 0, 0, 0, '', 0, '', '', 0, ?2)",
            )
            .bind(user)
            .bind(app)
            .execute(pool)
            .await
            .unwrap();
        }
        for (app, capabilities) in [
            ("encrypted-bot", r#"["CONTACT","ENCRYPTED"]"#),
            ("plain-bot", r#"["CONTACT"]"#),
        ] {
            crate::db::query(
                "INSERT INTO apps (app_id, app_number, home_uri, redirect_uri, name, icon_url, \
                 description, app_secret, capabilities, creator_id) \
                 VALUES (?1, '', '', '', '', '', '', '', ?2, '')",
            )
            .bind(app)
            .bind(capabilities)
            .execute(pool)
            .await
            .unwrap();
        }
        for (conversation, owner, member) in [
            ("with-friend", "friend", "me"),
            ("with-encrypted-bot", "encrypted-bot", "me"),
            ("with-plain-bot", "plain-bot", "me"),
            ("without-me", "friend", "friend"),
        ] {
            crate::db::query(
                "INSERT INTO conversations (conversation_id, owner_id, category, name, created_at, \
                 status, mute_until) VALUES (?, ?, 'CONTACT', '', 0, 0, 0)",
            )
            .bind(conversation)
            .bind(owner)
            .execute(pool)
            .await
            .unwrap();
            crate::db::query(
                "INSERT INTO participants (conversation_id, user_id, created_at) VALUES (?, ?, 0)",
            )
            .bind(conversation)
            .bind(member)
            .execute(pool)
            .await
            .unwrap();
        }
        signal
            .session_dao
            .save_session("me", 1, vec![1])
            .await
            .unwrap();

        service
            .process_system_message(&system_session("PROVISION", "phone"))
            .await
            .unwrap();

        let sessions = &database.participant_session_dao;
        assert!(!signal.session_dao.has_session("me").await.unwrap());
        for (conversation, expected) in [
            ("with-friend", true),
            ("with-encrypted-bot", true),
            ("with-plain-bot", false),
            ("without-me", false),
        ] {
            let rows = sessions
                .get_participant_sessions(conversation)
                .await
                .unwrap();
            assert_eq!(rows.iter().any(|row| row.session_id == "phone"), expected);
            assert!(rows
                .iter()
                .all(|row| row.public_key.as_deref() == Some("key")));
        }
    }

    #[tokio::test]
    async fn system_session_destroy_ignores_unknown_sessions_and_removes_known_ones() {
        let (_directory, database, signal, service) = decrypt_service_fixture().await;
        let sessions = &database.participant_session_dao;
        sessions
            .insert_session("conversation", "me", "phone", 1)
            .await
            .unwrap();
        signal
            .session_dao
            .save_session("me", 1, vec![1])
            .await
            .unwrap();

        service
            .process_system_message(&system_session("DESTROY", "unknown"))
            .await
            .unwrap();
        assert!(signal.session_dao.has_session("me").await.unwrap());
        assert!(sessions.has_user_session("me", "phone").await.unwrap());

        service
            .process_system_message(&system_session("DESTROY", "phone"))
            .await
            .unwrap();
        assert!(!signal.session_dao.has_session("me").await.unwrap());
        assert!(!sessions.has_user_session("me", "phone").await.unwrap());
    }

    #[tokio::test]
    async fn repeated_resend_request_from_a_session_queues_one_send() {
        let (_directory, database, _signal, service) = decrypt_service_fixture().await;
        let now = chrono::Utc::now();
        database
            .participant_dao
            .insert_participant(&Participant {
                conversation_id: "conversation".into(),
                user_id: "peer".into(),
                role: None,
                created_at: now - chrono::Duration::hours(1),
            })
            .await
            .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "mine".into(),
                conversation_id: "conversation".into(),
                user_id: "me".into(),
                category: "SIGNAL_TEXT".into(),
                content: Some("hello".into()),
                status: MessageStatus::Sent,
                created_at: now.naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        let pool = &database.job_dao.0;
        let payload = serde_json::json!({
            "action": RESEND_MESSAGES, "messages": ["mine", "absent"]
        })
        .to_string();
        let request = blaze_data("PLAIN_JSON", &Base64::encode_string(payload.as_bytes()));

        service.process_plain_message(&request).await.unwrap();
        let queued = crate::db::query_scalar::<i64>("SELECT COUNT(*) FROM jobs")
            .fetch_one(pool)
            .await
            .unwrap();
        // The job id repeats per message and session, so only a finished job
        // can show whether the second request was ignored.
        crate::db::query("DELETE FROM jobs")
            .execute(pool)
            .await
            .unwrap();
        let mut repeated = blaze_data("PLAIN_JSON", &Base64::encode_string(payload.as_bytes()));
        repeated.message_id = "event-2".into();
        service.process_plain_message(&repeated).await.unwrap();
        let requeued = crate::db::query_scalar::<i64>("SELECT COUNT(*) FROM jobs")
            .fetch_one(pool)
            .await
            .unwrap();

        assert_eq!((queued, requeued), (1, 0));
        let resends = &database.participant_session_dao;
        assert!(resends
            .has_resend_request("peer", PEER_SESSION, "absent")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn received_pin_restores_dismissed_preview_but_unpin_preserves_dismissal() {
        let (directory, database, signal_database, service) = decrypt_service_fixture().await;
        let app_database = crate::db::app::AppDatabase::connect_at(directory.path().join("app.db"))
            .await
            .unwrap();
        // Banner visibility lives in each saved account's Hive box.
        crate::db::hive::hydrated_box(directory.path())
            .unwrap()
            .put(
                "MultiAuthCubit",
                &serde_json::json!({"auths": [
                    {"account": {"user_id": "other", "identity_number": "7001"}},
                    {"account": {"user_id": "me", "identity_number": "7000"}},
                ]}),
            )
            .unwrap();
        let settings = app_database.setting_dao;
        let service = service.with_settings(settings.clone());
        let key = SettingDao::pinned_preview_key("me", "conversation");
        let other_key = SettingDao::pinned_preview_key("other", "conversation");
        settings.set(&key, Some("false")).await.unwrap();
        settings.set(&other_key, Some("false")).await.unwrap();
        let now = chrono::Utc::now();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "pinned-text".into(),
                conversation_id: "conversation".into(),
                user_id: "missing-sender".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("text".into()),
                status: MessageStatus::Read,
                created_at: now.naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();
        for (index, payload) in [
            PinMessagePayload::Pin(vec!["pinned-text".into()]),
            PinMessagePayload::Unpin(vec!["pinned-text".into()]),
            PinMessagePayload::Pin(vec!["pinned-text".into()]),
        ]
        .into_iter()
        .enumerate()
        {
            settings.set(&key, Some("false")).await.unwrap();
            let data = serde_json::from_value(serde_json::json!({
                "conversation_id":"conversation", "user_id":"missing-sender",
                "message_id": format!("pin-event-{index}"), "category":"MESSAGE_PIN",
                "data":Base64::encode_string(serde_json::to_string(&payload).unwrap().as_bytes()),
                "status":"SENT", "created_at":now, "updated_at":now,
                "source":"CREATE_MESSAGE", "session_id":""
            }))
            .unwrap();
            service.process_pin(&data).await.unwrap();
            assert_eq!(
                settings.get(&key).await.unwrap().as_deref(),
                // `ShowPinMessageKeyValue.show` stores true.
                if index == 1 {
                    Some("false")
                } else {
                    Some("true")
                }
            );
            assert_eq!(
                settings.get(&other_key).await.unwrap().as_deref(),
                Some("false")
            );
        }
        assert_eq!(
            database
                .pin_message_dao
                .message_ids("conversation")
                .await
                .unwrap(),
            ["pinned-text"]
        );
        drop(service);
        signal_database.close().await;
        database.close().await;
    }

    #[tokio::test]
    async fn missing_profiles_do_not_block_message_storage_or_quote_reads() {
        use chrono::Utc;

        let (_directory, database, signal_database, service) = decrypt_service_fixture().await;
        let now = Utc::now();
        for (id, category, content) in [
            ("text", "PLAIN_TEXT", "hello"),
            (
                "contact",
                "PLAIN_CONTACT",
                r#"{"user_id":"missing-contact"}"#,
            ),
            ("next", "PLAIN_TEXT", "following message"),
        ] {
            let data = serde_json::json!({"conversation_id":"conversation", "user_id":"missing-sender",
                "message_id":id, "category":category, "data":Base64::encode_string(content.as_bytes()),
                "status":"SENT", "created_at":now, "updated_at":now, "source":"CREATE_MESSAGE", "session_id":""});
            let flood = FloodMessage {
                message_id: id.into(),
                data: data.to_string(),
                created_at: now.naive_utc(),
            };
            tokio::time::timeout(Duration::from_secs(2), service.process_message(&flood))
                .await
                .unwrap()
                .unwrap();
            let stored = database
                .message_dao
                .find_message_by_id(&id.to_string())
                .await
                .unwrap()
                .unwrap();
            assert_ne!(stored.status, MessageStatus::Unknown);
            assert_eq!(stored.category, category);
            if id == "contact" {
                assert_eq!(stored.shared_user_id.as_deref(), Some("missing-contact"));
                assert_eq!(stored.name, None);
            } else {
                assert_eq!(stored.content.as_deref(), Some(content));
            }
            assert!(database
                .message_dao
                .find_quote_message_by_id(id)
                .await
                .unwrap()
                .is_some());
        }
        assert_eq!(
            service.app_service.conversation.pending_user_count().await,
            2
        );
        let items = database
            .conversation_dao
            .list_items("chats", None, "", false, 10, 0)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "missing-sender");
        assert_eq!(
            database
                .conversation_dao
                .count_items("chats", None, "", false)
                .await
                .unwrap(),
            1
        );
        signal_database.close().await;
        database.close().await;
    }

    #[tokio::test]
    async fn received_and_redecrypted_attachments_preserve_payload_metadata() {
        use chrono::{TimeZone, Utc};

        let (_directory, database, signal_database, service) = decrypt_service_fixture().await;
        let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let attachment_created_at = Utc.timestamp_opt(1_600_000_000, 0).unwrap();
        for (index, (category, duration, redecrypt)) in [
            (message_category::PLAIN_IMAGE, None, false),
            (message_category::SIGNAL_IMAGE, None, false),
            (message_category::ENCRYPTED_IMAGE, None, false),
            (message_category::PLAIN_AUDIO, Some(1234), false),
            (message_category::SIGNAL_AUDIO, Some(1234), false),
            (message_category::ENCRYPTED_AUDIO, Some(1234), false),
            (message_category::SIGNAL_IMAGE, None, true),
            (message_category::SIGNAL_AUDIO, Some(1234), true),
        ]
        .into_iter()
        .enumerate()
        {
            let message_id = format!("attachment-{index}");
            let attachment = AttachmentMessage {
                attachment_id: "server-attachment".into(),
                mime_type: if duration.is_some() {
                    "audio/ogg"
                } else {
                    "image/png"
                }
                .into(),
                size: 1024,
                name: Some("attachment".into()),
                width: Some(320),
                height: Some(240),
                thumbnail: Some("thumbnail".into()),
                duration,
                waveform: Some(vec![1, 2, 3]),
                caption: Some("A caption from the sender".into()),
                key: Some(vec![0; 32]),
                digest: Some(vec![1; 32]),
                created_at: Some(attachment_created_at),
                shareable: Some(true),
            };
            let json = serde_json::to_string(&attachment).unwrap();
            let plain_text = if category.starts_with("ENCRYPTED_") {
                json
            } else {
                Base64::encode_string(json.as_bytes())
            };
            let data: BlazeMessageData = serde_json::from_value(serde_json::json!({
                "conversation_id": "conversation", "user_id": "missing-sender",
                "message_id": message_id, "category": category, "data": plain_text,
                "status": "SENT", "created_at": now, "updated_at": now,
                "source": "CREATE_MESSAGE", "session_id": ""
            }))
            .unwrap();
            if redecrypt {
                database
                    .message_dao
                    .insert_message(&Message {
                        message_id: message_id.clone(),
                        conversation_id: data.conversation_id.clone(),
                        user_id: data.sender_id().clone(),
                        category: category.into(),
                        status: MessageStatus::Failed,
                        created_at: now.naive_utc(),
                        ..Message::default()
                    })
                    .await
                    .unwrap();
                service
                    .process_re_decrypted_message(&data, &message_id, &plain_text)
                    .await
                    .unwrap();
            } else {
                service
                    .process_decrypt_success(&data, &plain_text)
                    .await
                    .unwrap();
            }
            let stored = database
                .message_dao
                .find_message_by_id(&message_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                stored.caption, attachment.caption,
                "{category}, redecrypt={redecrypt}"
            );
            assert_eq!(
                stored.media_duration,
                duration.map(|value| value.to_string()).unwrap_or_default()
            );
            assert_eq!(stored.media_key, attachment.key);
            assert_eq!(stored.media_digest, attachment.digest);
            assert_eq!(
                stored.media_waveform,
                attachment.waveform.as_deref().map(Base64::encode_string)
            );
            assert_eq!(
                stored.media_mime_type.as_deref(),
                Some(attachment.mime_type.as_str())
            );
            assert_eq!(stored.media_size, Some(attachment.size));
            assert_eq!(stored.name, attachment.name);
            assert_eq!(stored.media_width, attachment.width);
            assert_eq!(stored.media_height, attachment.height);
            assert_eq!(stored.thumb_image, attachment.thumbnail);
            assert_eq!(stored.status, MessageStatus::Sent);
            let extra: AttachmentExtra =
                serde_json::from_str(stored.content.as_deref().unwrap()).unwrap();
            assert_eq!(extra.attachment_id, attachment.attachment_id);
            assert_eq!(extra.message_id, message_id);
            assert_eq!(extra.shareable, attachment.shareable);
            assert_eq!(
                extra.created_at, attachment.created_at,
                "{category}, redecrypt={redecrypt}"
            );
        }
        signal_database.close().await;
        database.close().await;
    }

    fn fts_message(category: &str, content: Option<&str>) -> Message {
        Message {
            category: category.to_string(),
            content: content.map(str::to_string),
            status: MessageStatus::Sent,
            ..Message::default()
        }
    }

    #[test]
    fn builds_category_specific_fts_content() {
        assert_eq!(
            message_fts_content(&fts_message(message_category::PLAIN_TEXT, Some("hello")))
                .as_deref(),
            Some("hello")
        );

        let mut data = fts_message(message_category::PLAIN_DATA, Some("attachment-json"));
        data.name = Some("document.pdf".into());
        assert_eq!(message_fts_content(&data).as_deref(), Some("document.pdf"));

        let mut contact = fts_message(message_category::PLAIN_CONTACT, Some("contact-json"));
        contact.name = Some("Mixin User".into());
        assert_eq!(message_fts_content(&contact).as_deref(), Some("Mixin User"));

        let card = fts_message(
            message_category::APP_CARD,
            Some(r#"{"app_id":"app","title":"Card title","description":"Card body"}"#),
        );
        assert_eq!(
            message_fts_content(&card).as_deref(),
            Some("Card title Card body")
        );
    }

    #[test]
    fn skips_failed_unknown_and_transcript_summary_content() {
        let mut failed = fts_message(message_category::PLAIN_TEXT, Some("ciphertext"));
        failed.status = MessageStatus::Failed;
        assert!(message_fts_content(&failed).is_none());

        let mut unknown = fts_message(message_category::PLAIN_TEXT, Some("unknown"));
        unknown.status = MessageStatus::Unknown;
        assert!(message_fts_content(&unknown).is_none());

        assert!(message_fts_content(&fts_message(
            message_category::PLAIN_TRANSCRIPT,
            Some("summary")
        ))
        .is_none());
    }
}
