use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use base64ct::{Base64, Base64UrlUnpadded, Encoding};
use log::error;
use serde::Deserialize;
use tokio::sync::Notify;

use sdk::err::error_code::BAD_DATA;
use sdk::message::RecallMessage;
use sdk::message_category::MessageCategory;
use sdk::{
    message_category, AttachmentMessage, BlazeMessage, BlazeMessageParam, Client, MessageStatus,
    PIN_MESSAGE, RECALL_MESSAGE, SENDING_MESSAGE,
};

use crate::core::attachment::{attachment_path, transcript_attachment_path};
use crate::core::conversation_change::ConversationChangeNotifier;
use crate::core::crypto::encrypted_protocol;
use crate::core::message::sender::{MessageResult, MessageSender};
use crate::core::model::{AttachmentExtra, ConversationService};
use crate::db::mixin::job::Job;
use crate::db::mixin::message::Message;
use crate::db::mixin::participant_session::ParticipantSession;
use crate::db::path::account_data_directory;
use crate::db::MixinDatabase;

use super::{is_terminal_result, sanitize_transcript_app_card, JobCategory, JobTrigger};

const MAX_TEXT_LENGTH: usize = 64 * 1024;

pub(super) struct SendingJobRunner {
    pub(super) database: Arc<MixinDatabase>,
    pub(super) client: Arc<Client>,
    pub(super) user_id: String,
    pub(super) session_id: String,
    pub(super) identity_number: String,
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
                    RECALL_MESSAGE => self.send_recall_message(&job).await,
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

    async fn send_recall_message(&self, job: &Job) -> Result<bool> {
        let conversation_id = job
            .conversation_id
            .as_deref()
            .ok_or_else(|| anyhow!("recall job has no conversation id"))?;
        let content = job
            .blaze_message
            .as_deref()
            .ok_or_else(|| anyhow!("recall job has no payload"))?;
        let recall: RecallMessage =
            serde_json::from_str(content).context("decode recall job payload")?;
        if job.run_count > 0 {
            self.finish_recall_job(job, &recall).await?;
            return Ok(false);
        }

        let result = self
            .sender
            .deliver(BlazeMessage::new_param_blaze(BlazeMessageParam {
                conversation_id: Some(conversation_id.to_string()),
                conversation_checksum: Some(self.sender.get_check_sum(conversation_id).await?),
                message_id: Some(job.job_id.clone()),
                category: Some(message_category::MESSAGE_RECALL.to_string()),
                data: Some(Base64::encode_string(content.as_bytes())),
                status: Some(MessageStatus::Sending.into()),
                ..BlazeMessageParam::default()
            }))
            .await?;
        if result.success && result.error_code.is_none() {
            self.database
                .job_dao
                .reschedule_job(&job.job_id, chrono::Utc::now().naive_utc(), 1)
                .await?;
            self.finish_recall_job(job, &recall).await?;
            return Ok(false);
        }
        if is_terminal_result(&result) {
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(false);
        }
        Ok(true)
    }

    async fn finish_recall_job(&self, job: &Job, recall: &RecallMessage) -> Result<()> {
        let conversation_id = job
            .conversation_id
            .as_deref()
            .ok_or_else(|| anyhow!("recall job has no conversation id"))?;
        let Some(message) = self
            .database
            .message_dao
            .find_message_by_id(&recall.message_id)
            .await?
        else {
            self.database.job_dao.delete_job_by_id(&job.job_id).await?;
            return Ok(());
        };
        if !message.category.is_recall() {
            for path in self.recalled_attachment_paths(&message).await? {
                match crate::fs::remove_file(&path).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("remove recalled attachment {}", path.display())
                        });
                    }
                }
            }
            self.database
                .message_dao
                .recall_message(conversation_id, &recall.message_id)
                .await?;
        }
        self.database.job_dao.delete_job_by_id(&job.job_id).await?;
        self.notify_changes(conversation_id);
        Ok(())
    }

    async fn recalled_attachment_paths(&self, message: &Message) -> Result<Vec<PathBuf>> {
        let account_data_dir = account_data_directory(&self.identity_number)?;
        let mut paths = Vec::new();
        if message.category.is_transcript() {
            for transcript in self
                .database
                .transcript_message_dao
                .find_by_transcript_id(&message.message_id)
                .await?
                .into_iter()
                .filter(|transcript| transcript.category.is_attachment())
            {
                let Some(media_url) = transcript
                    .media_url
                    .as_deref()
                    .filter(|media_url| !media_url.trim().is_empty())
                else {
                    continue;
                };
                let path = if Path::new(media_url).is_absolute() {
                    PathBuf::from(media_url)
                } else {
                    transcript_attachment_path(
                        &account_data_dir,
                        &Message {
                            message_id: transcript.message_id,
                            conversation_id: message.conversation_id.clone(),
                            category: transcript.category,
                            media_mime_type: transcript.media_mime_type,
                            name: transcript.media_name,
                            ..Message::default()
                        },
                    )?
                };
                paths.push(path);
            }
        } else if message.category.is_attachment() {
            if let Some(media_url) = message
                .media_url
                .as_deref()
                .filter(|media_url| !media_url.trim().is_empty())
            {
                paths.push(if Path::new(media_url).is_absolute() {
                    PathBuf::from(media_url)
                } else {
                    attachment_path(&account_data_dir, message)?
                });
            }
        }
        paths.sort_unstable();
        paths.dedup();
        Ok(paths)
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
        // A durable job may be polled while attachment uploads are still running.
        // Keep it queued until all attachment credentials have been persisted.
        if (message.category.is_transcript() || message.category.is_attachment())
            && message.media_status != crate::db::mixin::message::MediaStatus::Done
        {
            return Ok(true);
        }
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
            transcript_sending_content(&mut transcripts)?
        } else if resend && message.category.is_attachment() {
            resend_attachment_content(&message, stored_content)?
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
        let mentions = outgoing_mentions(&self.database, &message.category, content).await?;
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
                &payload,
                content,
                mentions.as_deref(),
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
                .send_encrypted_message(&message, &payload, content, mentions.as_deref(), expire_in)
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
                        &payload,
                        content,
                        mentions.as_deref(),
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
        payload: &SendingPayload,
        content: &str,
        mentions: Option<&[String]>,
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
                recipient_id: payload.recipient_id.clone(),
                message_id: Some(message.message_id.clone()),
                category: Some(category.clone()),
                data: Some(encoded),
                quote_message_id: message.quote_message_id.clone(),
                mentions: mentions.map(<[String]>::to_vec),
                silent: Some(payload.silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            }))
            .await
    }

    /// A message addressed to one bot in a group encrypts to that bot's key.
    async fn encrypted_recipient_session(
        &self,
        message: &crate::db::mixin::message::Message,
        payload: &SendingPayload,
    ) -> Result<Option<ParticipantSession>> {
        let sessions = &self.database.participant_session_dao;
        Ok(match &payload.recipient_id {
            Some(user_id) => {
                sessions
                    .participant_session_key_of_user(&message.conversation_id, user_id)
                    .await?
            }
            None => {
                sessions
                    .participant_session_key_without_self(&message.conversation_id, &self.user_id)
                    .await?
            }
        })
    }

    async fn send_encrypted_message(
        &self,
        message: &crate::db::mixin::message::Message,
        payload: &SendingPayload,
        content: &str,
        mentions: Option<&[String]>,
        expire_in: i32,
    ) -> Result<Option<MessageResult>> {
        let mut remote = self.encrypted_recipient_session(message, payload).await?;
        if remote.is_none() {
            ConversationService::new(
                self.database.clone(),
                self.client.clone(),
                self.user_id.clone(),
            )
            .refresh_conversation(&message.conversation_id)
            .await?;
            remote = self.encrypted_recipient_session(message, payload).await?;
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
            decode_session_public_key(
                remote
                    .public_key
                    .as_deref()
                    .ok_or_else(|| anyhow!("recipient session has no public key"))?,
            )?,
        )];
        if let Some(other) = other_self {
            sessions.push((
                other.session_id,
                decode_session_public_key(
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
                recipient_id: payload.recipient_id.clone(),
                message_id: Some(message.message_id.clone()),
                category: Some(message.category.clone()),
                data: Some(Base64::encode_string(&encrypted)),
                quote_message_id: message.quote_message_id.clone(),
                mentions: mentions.map(<[String]>::to_vec),
                silent: Some(payload.silent),
                expire_in: Some(expire_in),
                ..BlazeMessageParam::default()
            }))
            .await?;
        Ok(Some(result))
    }
}

/// A sent attachment stores only its `AttachmentExtra`; a resend must carry the
/// payload that the recipient decodes. Other content already is that payload.
fn resend_attachment_content(message: &Message, stored_content: &str) -> Result<String> {
    let Ok(extra) = serde_json::from_str::<AttachmentExtra>(stored_content) else {
        return Ok(stored_content.to_string());
    };
    let attachment = AttachmentMessage {
        key: message.media_key.clone(),
        digest: message.media_digest.clone(),
        attachment_id: extra.attachment_id,
        mime_type: message.media_mime_type.clone().unwrap_or_default(),
        size: message.media_size.unwrap_or_default(),
        name: message.name.clone(),
        width: message.media_width,
        height: message.media_height,
        thumbnail: message.thumb_image.clone(),
        duration: message.media_duration.parse().ok(),
        waveform: message
            .media_waveform
            .as_deref()
            .and_then(|value| Base64::decode_vec(value).ok()),
        caption: message.caption.clone(),
        created_at: extra.created_at,
        shareable: extra.shareable,
    };
    Ok(Base64::encode_string(
        serde_json::to_string(&attachment)?.as_bytes(),
    ))
}

/// Session public keys arrive in URL-safe Base64 without padding, the form
/// both mobile clients decode.
fn decode_session_public_key(value: &str) -> Result<Vec<u8>> {
    Ok(Base64UrlUnpadded::decode_vec(value.trim_end_matches('='))?)
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

async fn outgoing_mentions(
    database: &MixinDatabase,
    category: &String,
    content: &str,
) -> Result<Option<Vec<String>>> {
    if !category.is_text() {
        return Ok(None);
    }
    let user_ids = database
        .user_dao
        .find_user_ids_by_identity_numbers(&mention_identity_numbers(content))
        .await?;
    Ok((!user_ids.is_empty()).then_some(user_ids))
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

fn transcript_sending_content(
    transcripts: &mut [crate::db::mixin::transcript_message::TranscriptMessage],
) -> Result<String> {
    let wire = transcripts
        .iter_mut()
        .map(|transcript| {
            sanitize_transcript_app_card(transcript);
            if transcript.category.is_attachment() {
                if transcript.media_status != Some(crate::db::mixin::message::MediaStatus::Done) {
                    bail!(
                        "transcript attachment is not uploaded: {}",
                        transcript.message_id
                    );
                }
                transcript.content = Some(crate::core::message::decrypt::transcript_attachment_id(
                    transcript.content.as_deref().unwrap_or_default(),
                )?);
            }
            let mut value = serde_json::to_value(transcript)?;
            if let Some(object) = value.as_object_mut() {
                object.remove("media_url");
            }
            Ok(value)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::to_string(&wire)?)
}

#[cfg(test)]
mod tests {
    use super::{encrypted_to_plain_category, mention_identity_numbers, truncate_utf16};

    #[test]
    fn transcript_send_requires_uploaded_children_and_matches_flutter_wire_contract() {
        use crate::db::mixin::{message::MediaStatus, transcript_message::TranscriptMessage};
        let mut child: TranscriptMessage = serde_json::from_value(serde_json::json!({
            "transcript_id":"root", "message_id":"audio", "category":"SIGNAL_AUDIO",
            "created_at":"2024-01-02T03:04:05Z", "content":"{\"attachment_id\":\"uploaded-id\",\"message_id\":\"audio\"}",
            "media_url":"/private/account/source.ogg", "media_status":"PENDING",
            "media_duration":"1200", "media_key":"key", "media_digest":"digest"
        })).unwrap();
        assert!(super::transcript_sending_content(&mut [child.clone()]).is_err());
        child.media_status = Some(MediaStatus::Done);
        let wire = super::transcript_sending_content(&mut [child]).unwrap();
        let wire: serde_json::Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(wire[0]["content"], "uploaded-id");
        assert_eq!(wire[0]["category"], "SIGNAL_AUDIO");
        assert_eq!(wire[0]["media_duration"], 1200);
        assert_eq!(wire[0]["media_key"], "key");
        assert_eq!(wire[0]["media_digest"], "digest");
        assert!(wire[0].get("media_url").is_none());
        assert!(wire[0].get("media_status").is_none());
    }

    #[test]
    fn session_public_key_decodes_url_safe_form_and_rejects_standard_alphabet() {
        use base64ct::{Base64, Base64UrlUnpadded, Encoding};
        let key = [0xfb_u8; 32];
        let url_safe = Base64UrlUnpadded::encode_string(&key);
        assert!(url_safe.contains('-') && url_safe.contains('_'));

        assert_eq!(super::decode_session_public_key(&url_safe).unwrap(), key);
        assert_eq!(
            super::decode_session_public_key(&format!("{url_safe}=")).unwrap(),
            key
        );
        assert!(super::decode_session_public_key(&Base64::encode_string(&key)).is_err());
    }

    #[test]
    fn resent_attachment_carries_decodable_payload_rebuilt_from_stored_row() {
        use base64ct::{Base64, Encoding};
        let message = crate::db::mixin::message::Message {
            message_id: "image".into(),
            category: "SIGNAL_IMAGE".into(),
            media_key: Some(vec![1, 2]),
            media_digest: Some(vec![3]),
            media_mime_type: Some("image/png".into()),
            media_size: Some(42),
            media_width: Some(8),
            media_duration: "1200".into(),
            ..Default::default()
        };
        let stored = r#"{"attachment_id":"uploaded","message_id":"image","shareable":false}"#;

        let wire = super::resend_attachment_content(&message, stored).unwrap();

        let payload: sdk::AttachmentMessage =
            serde_json::from_slice(&Base64::decode_vec(&wire).unwrap()).unwrap();
        assert_eq!(payload.attachment_id, "uploaded");
        assert_eq!(payload.key, Some(vec![1, 2]));
        assert_eq!(payload.digest, Some(vec![3]));
        assert_eq!(payload.mime_type, "image/png");
        assert_eq!((payload.size, payload.width), (42, Some(8)));
        assert_eq!(payload.duration, Some(1200));
        assert_eq!(payload.shareable, Some(false));
        assert_eq!(
            super::resend_attachment_content(&message, "already-a-payload").unwrap(),
            "already-a-payload"
        );
    }

    #[tokio::test]
    async fn outgoing_mentions_cover_every_text_encryption_and_skip_other_categories() {
        let directory = tempfile::tempdir().unwrap();
        let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query(
            "INSERT INTO users (user_id, identity_number, full_name, avatar_url, phone, \
             is_verified, created_at, mute_until, has_pin, biography, is_scam, code_url, \
             code_id, is_deactivated) \
             VALUES ('alice', '7000', '', '', '', 0, 0, 0, 0, '', 0, '', '', 0)",
        )
        .execute(&database.user_dao.0)
        .await
        .unwrap();
        for category in ["SIGNAL_TEXT", "PLAIN_TEXT", "ENCRYPTED_TEXT"] {
            assert_eq!(
                super::outgoing_mentions(&database, &category.to_owned(), "hi @7000")
                    .await
                    .unwrap(),
                Some(vec!["alice".to_owned()]),
                "{category}"
            );
        }
        assert_eq!(
            super::outgoing_mentions(&database, &"PLAIN_POST".to_owned(), "hi @7000")
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            super::outgoing_mentions(&database, &"PLAIN_TEXT".to_owned(), "nobody @9999")
                .await
                .unwrap(),
            None
        );
        database.close().await;
    }

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
