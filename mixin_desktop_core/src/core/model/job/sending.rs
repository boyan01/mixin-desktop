use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Encoding};
use log::error;
use serde::Deserialize;
use tokio::sync::Notify;

use sdk::err::error_code::BAD_DATA;
use sdk::message_category::MessageCategory;
use sdk::{
    message_category, AttachmentMessage, BlazeMessage, BlazeMessageParam, Client, MessageStatus,
    PIN_MESSAGE, RECALL_MESSAGE, SENDING_MESSAGE,
};

use crate::core::conversation_change::ConversationChangeNotifier;
use crate::core::crypto::encrypted_protocol;
use crate::core::message::sender::{MessageResult, MessageSender};
use crate::core::model::{AttachmentExtra, ConversationService};
use crate::db::mixin::job::Job;
use crate::db::MixinDatabase;

use super::{is_terminal_result, sanitize_transcript_app_card, JobCategory, JobTrigger};

const MAX_TEXT_LENGTH: usize = 64 * 1024;

pub(super) struct SendingJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) client: Arc<Client>,
    pub(super) user_id: String,
    pub(super) session_id: String,
    pub(super) private_key: Vec<u8>,
    pub(super) sender: Arc<MessageSender>,
    pub(super) changes: Option<ConversationChangeNotifier>,
    pub(super) expired_message_notify: Arc<Notify>,
}

#[derive(Debug, Deserialize)]
struct SendingPayload {
    message_id: String,
    recipient_id: Option<String>,
    session_id: Option<String>,
    #[serde(default)]
    resend: bool,
    #[serde(default)]
    silent: bool,
    #[serde(default, alias = "expireIn")]
    expire_in: i64,
}

impl JobTrigger for SendingJobRunner {
    async fn trigger(&self) -> Result<bool> {
        self.run_sending_jobs().await
    }

    fn category(&self) -> JobCategory {
        JobCategory::Sending
    }
}

impl SendingJobRunner {
    fn notify_changes(&self, conversation_id: &str) {
        if let Some(changes) = &self.changes {
            changes.notify(conversation_id);
        }
    }

    async fn run_sending_jobs(&self) -> Result<bool> {
        loop {
            let jobs = self.database.job_dao.sending_jobs().await?;
            if jobs.is_empty() {
                return Ok(false);
            }
            let mut retry = false;
            for job in jobs {
                let result = match job.action.as_str() {
                    PIN_MESSAGE => {
                        self.send_control_message(&job, message_category::MESSAGE_PIN)
                            .await
                    }
                    RECALL_MESSAGE => {
                        self.send_control_message(&job, message_category::MESSAGE_RECALL)
                            .await
                    }
                    SENDING_MESSAGE => self.send_user_message(&job).await,
                    _ => Ok(false),
                };
                match result {
                    Ok(job_retry) => retry |= job_retry,
                    Err(err) => {
                        error!("failed to run sending job {}: {err}", job.job_id);
                        retry = true;
                    }
                }
            }
            if retry {
                return Ok(true);
            }
        }
    }

    async fn send_control_message(&self, job: &Job, category: &str) -> Result<bool> {
        let conversation_id = job
            .conversation_id
            .as_deref()
            .ok_or_else(|| anyhow!("control job has no conversation id"))?;
        let content = job
            .blaze_message
            .as_deref()
            .ok_or_else(|| anyhow!("control job has no payload"))?;
        let result = self
            .sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(conversation_id.to_string()),
                conversation_checksum: Some(self.sender.get_check_sum(conversation_id).await?),
                message_id: Some(uuid::Uuid::new_v4().to_string()),
                category: Some(category.to_string()),
                data: Some(Base64::encode_string(content.as_bytes())),
                status: Some(MessageStatus::Sending.into()),
                ..BlazeMessageParam::default()
            }))
            .await?;
        if is_terminal_result(&result) {
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(false);
        }
        Ok(true)
    }

    async fn send_user_message(&self, job: &Job) -> Result<bool> {
        let payload_text = job
            .blaze_message
            .as_deref()
            .ok_or_else(|| anyhow!("sending job has no payload"))?;
        let payload = serde_json::from_str::<SendingPayload>(payload_text).unwrap_or_else(|_| {
            SendingPayload {
                message_id: payload_text.to_string(),
                recipient_id: job.user_id.clone(),
                session_id: None,
                resend: job.resend_message_id.is_some(),
                silent: false,
                expire_in: 0,
            }
        });
        let resend = payload.resend || job.resend_message_id.is_some();
        let message = if resend {
            self.database
                .message_dao
                .find_message_by_id(&payload.message_id)
                .await?
        } else {
            self.database
                .message_dao
                .find_sending_message(&payload.message_id)
                .await?
        };
        let Some(message) = message else {
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(false);
        };
        let Some(conversation) = self
            .database
            .conversation_dao
            .find_conversation_by_id(&message.conversation_id)
            .await?
        else {
            error!("conversation {} not found", message.conversation_id);
            return Ok(true);
        };
        let conversation_service = ConversationService::new(
            self.database.clone(),
            self.client.clone(),
            self.user_id.clone(),
        );
        if let Err(error) = conversation_service
            .ensure_conversation_exists(&conversation)
            .await
        {
            if error.downcast_ref::<sdk::ApiError>().is_some_and(
                |error| matches!(error, sdk::ApiError::Server(error) if error.code == BAD_DATA),
            ) {
                if !resend {
                    self.database
                        .message_dao
                        .complete_sending_job(
                            &message.message_id,
                            None,
                            MessageStatus::Failed,
                            0,
                            &job.job_id,
                        )
                        .await?;
                } else {
                    self.database.job_dao.delete_job_by_id(&job.job_id).await?;
                }
                self.notify_changes(&message.conversation_id);
                return Ok(false);
            }
            return Err(error);
        }
        let stored_content = message
            .content
            .as_deref()
            .ok_or_else(|| anyhow!("message {} has no content", message.message_id))?;
        let mut outbound_content = if message.category.is_transcript() {
            let mut transcripts = self
                .database
                .transcript_message_dao
                .find_by_transcript_id(&message.message_id)
                .await?;
            if transcripts.is_empty() {
                bail!("message {} has no transcript entries", message.message_id);
            }
            for transcript in &mut transcripts {
                sanitize_transcript_app_card(transcript);
            }
            serde_json::to_string(&transcripts)?
        } else {
            stored_content.to_string()
        };
        let mut sent_content = None;
        if message.category.is_text() || message.category.is_post() {
            outbound_content = truncate_utf16(&outbound_content, MAX_TEXT_LENGTH).to_string();
            sent_content = Some(outbound_content.clone());
        } else if message.category.is_attachment() {
            if let Ok(decoded) = Base64::decode_vec(&outbound_content) {
                if let Ok(attachment) = serde_json::from_slice::<AttachmentMessage>(&decoded) {
                    sent_content = Some(serde_json::to_string(&AttachmentExtra {
                        attachment_id: attachment.attachment_id,
                        message_id: message.message_id.clone(),
                        shareable: attachment.shareable,
                        created_at: attachment.created_at,
                    })?);
                }
            }
        }
        let content = outbound_content.as_str();
        let mentions = if message.category.is_signal() && message.category.is_text() {
            let identity_numbers = mention_identity_numbers(content);
            let user_ids = self
                .database
                .user_dao
                .find_user_ids_by_identity_numbers(&identity_numbers)
                .await?;
            (!user_ids.is_empty()).then_some(user_ids)
        } else {
            None
        };
        if payload.expire_in < 0 {
            self.database
                .message_dao
                .complete_sending_job(
                    &message.message_id,
                    None,
                    MessageStatus::Failed,
                    0,
                    &job.job_id,
                )
                .await?;
            self.notify_changes(&message.conversation_id);
            return Ok(false);
        }
        let expire_in = i32::try_from(payload.expire_in)
            .with_context(|| format!("invalid expire_in: {}", payload.expire_in))?;

        let result = if message.category.is_plain()
            || message.category.is_app_card()
            || message.category.is_app_button_group()
        {
            self.send_plain_message(
                &message,
                &message.category,
                payload.recipient_id.as_deref(),
                content,
                payload.silent,
                expire_in,
            )
            .await?
        } else if message.category.is_signal() {
            let content = if message.category.is_live() {
                Base64::encode_string(content.as_bytes())
            } else {
                content.to_string()
            };
            if resend {
                self.sender
                    .resend_signal_message(
                        &message.message_id,
                        &message.conversation_id,
                        payload
                            .recipient_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("resend job has no recipient"))?,
                        payload
                            .session_id
                            .as_deref()
                            .ok_or_else(|| anyhow!("resend job has no session"))?,
                        &message.category,
                        &content,
                        message.quote_message_id.as_deref(),
                        mentions.as_deref(),
                        payload.silent,
                        expire_in,
                    )
                    .await?
            } else {
                self.sender
                    .send_signal_message(
                        &message.message_id,
                        &message.conversation_id,
                        &message.category,
                        &content,
                        message.quote_message_id.as_deref(),
                        mentions.as_deref(),
                        payload.silent,
                        expire_in,
                    )
                    .await?
            }
        } else if message.category.is_encrypted() {
            match self
                .send_encrypted_message(&message, &payload, content, expire_in)
                .await?
            {
                Some(result) => result,
                None => {
                    let category =
                        encrypted_to_plain_category(&message.category).ok_or_else(|| {
                            anyhow!("invalid encrypted category: {}", message.category)
                        })?;
                    self.database
                        .message_dao
                        .update_message_category(&message.message_id, &category)
                        .await?;
                    self.send_plain_message(
                        &message,
                        &category,
                        payload.recipient_id.as_deref(),
                        content,
                        payload.silent,
                        expire_in,
                    )
                    .await?
                }
            }
        } else {
            self.database
                .message_dao
                .complete_sending_job(
                    &message.message_id,
                    None,
                    MessageStatus::Failed,
                    0,
                    &job.job_id,
                )
                .await?;
            self.notify_changes(&message.conversation_id);
            return Ok(false);
        };

        if is_terminal_result(&result) {
            if resend {
                self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            } else {
                let status = if result.success {
                    MessageStatus::Sent
                } else {
                    MessageStatus::Failed
                };
                self.database
                    .message_dao
                    .complete_sending_job(
                        &message.message_id,
                        result.success.then_some(sent_content).flatten().as_deref(),
                        status,
                        payload.expire_in,
                        &job.job_id,
                    )
                    .await?;
                if result.success && payload.expire_in > 0 {
                    self.expired_message_notify.notify_one();
                }
            }
            self.notify_changes(&message.conversation_id);
            return Ok(false);
        }
        Ok(true)
    }

    async fn send_plain_message(
        &self,
        message: &crate::db::mixin::message::Message,
        category: &String,
        recipient_id: Option<&str>,
        content: &str,
        silent: bool,
        expire_in: i32,
    ) -> Result<MessageResult> {
        let encoded = if category.is_text()
            || category.is_post()
            || category.is_live()
            || category.is_location()
            || category.is_transcript()
            || category.is_app_card()
            || category.is_app_button_group()
        {
            Base64::encode_string(content.as_bytes())
        } else {
            content.to_string()
        };
        self.sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(message.conversation_id.clone()),
                conversation_checksum: Some(
                    self.sender.get_check_sum(&message.conversation_id).await?,
                ),
                recipient_id: recipient_id.map(str::to_string),
                message_id: Some(message.message_id.clone()),
                category: Some(category.clone()),
                data: Some(encoded),
                quote_message_id: message.quote_message_id.clone(),
                silent: Some(silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            }))
            .await
    }

    async fn send_encrypted_message(
        &self,
        message: &crate::db::mixin::message::Message,
        payload: &SendingPayload,
        content: &str,
        expire_in: i32,
    ) -> Result<Option<MessageResult>> {
        let mut remote = self
            .database
            .participant_session_dao
            .participant_session_key_without_self(&message.conversation_id, &self.user_id)
            .await?;
        if remote.is_none() {
            ConversationService::new(
                self.database.clone(),
                self.client.clone(),
                self.user_id.clone(),
            )
            .refresh_conversation(&message.conversation_id)
            .await?;
            remote = self
                .database
                .participant_session_dao
                .participant_session_key_without_self(&message.conversation_id, &self.user_id)
                .await?;
        }
        let Some(remote) = remote else {
            return Ok(None);
        };
        let other_self = self
            .database
            .participant_session_dao
            .other_participant_session_key(
                &message.conversation_id,
                &self.user_id,
                &self.session_id,
            )
            .await?;

        let mut sessions = vec![(
            remote.session_id,
            Base64::decode_vec(
                remote
                    .public_key
                    .as_deref()
                    .ok_or_else(|| anyhow!("recipient session has no public key"))?,
            )?,
        )];
        if let Some(other) = other_self {
            sessions.push((
                other.session_id,
                Base64::decode_vec(
                    other
                        .public_key
                        .as_deref()
                        .ok_or_else(|| anyhow!("own session has no public key"))?,
                )?,
            ));
        }
        let session_refs = sessions
            .iter()
            .map(|(session_id, key)| (session_id.as_str(), key.as_slice()))
            .collect::<Vec<_>>();
        let plaintext = if message.category.is_attachment()
            || message.category.is_sticker()
            || message.category.is_contact()
        {
            Base64::decode_vec(content)?
        } else {
            content.as_bytes().to_vec()
        };
        let encrypted =
            encrypted_protocol::encrypt_message(&self.private_key, &plaintext, &session_refs)?;
        let result = self
            .sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(message.conversation_id.clone()),
                conversation_checksum: Some(
                    self.sender.get_check_sum(&message.conversation_id).await?,
                ),
                message_id: Some(message.message_id.clone()),
                category: Some(message.category.clone()),
                data: Some(Base64::encode_string(&encrypted)),
                quote_message_id: message.quote_message_id.clone(),
                silent: Some(payload.silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            }))
            .await?;
        Ok(Some(result))
    }
}

fn truncate_utf16(value: &str, max_code_units: usize) -> &str {
    if value.encode_utf16().count() <= max_code_units {
        return value;
    }
    let mut code_units = 0;
    let mut end = 0;
    for (index, character) in value.char_indices() {
        let next = code_units + character.len_utf16();
        if next > max_code_units {
            break;
        }
        code_units = next;
        end = index + character.len_utf8();
    }
    &value[..end]
}

fn mention_identity_numbers(content: &str) -> Vec<String> {
    let mut identities = Vec::new();
    for (index, _) in content.match_indices('@') {
        let identity = content[index + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if identity.len() >= 4 && !identities.contains(&identity) {
            identities.push(identity);
        }
    }
    identities
}

fn encrypted_to_plain_category(category: &str) -> Option<String> {
    category
        .strip_prefix("ENCRYPTED_")
        .map(|suffix| format!("PLAIN_{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::{encrypted_to_plain_category, mention_identity_numbers, truncate_utf16};

    #[test]
    fn truncates_using_dart_utf16_length_semantics() {
        assert_eq!(truncate_utf16("ab😀c", 4), "ab😀");
        assert_eq!(truncate_utf16("ab😀c", 3), "ab");
        assert_eq!(truncate_utf16("short", 10), "short");
    }

    #[test]
    fn extracts_unique_flutter_style_mentions() {
        assert_eq!(
            mention_identity_numbers("hi @1234 @12 @1234 and @56789x"),
            vec!["1234", "56789"]
        );
    }

    #[test]
    fn maps_encrypted_category_to_flutter_plain_fallback() {
        assert_eq!(
            encrypted_to_plain_category("ENCRYPTED_TRANSCRIPT").as_deref(),
            Some("PLAIN_TRANSCRIPT")
        );
        assert_eq!(encrypted_to_plain_category("SIGNAL_TEXT"), None);
    }
}
