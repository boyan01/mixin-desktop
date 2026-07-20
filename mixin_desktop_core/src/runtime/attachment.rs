use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use base64ct::{Base64, Encoding};
use sdk::message_category::MessageCategory as _;
use sdk::{AttachmentMessage, MessageStatus};
use tokio_util::sync::CancellationToken;

use crate::core::attachment::{attachment_file_name, attachment_path, transcript_attachment_path};
use crate::core::message::decrypt::{transcript_attachment_id, transcript_attachment_message};
use crate::core::model::AttachmentExtra;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::AttachmentMessageUpdate;
use crate::db::mixin::message::MediaStatus;
use crate::db::path::account_data_directory;

use super::{transcript_download_key, AccountState, MessageAccess};

pub struct AttachmentAccess {
    state: Arc<AccountState>,
}

impl AttachmentAccess {
    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }
}

impl Deref for AttachmentAccess {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl AttachmentAccess {
    pub async fn retry_attachment(&self, message_id: String) -> Result<()> {
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?;
        if message.user_id != self.account_id {
            return Err(anyhow!("only outgoing attachments can be retried"));
        }
        if !message.category.is_attachment() {
            return Err(anyhow!("message is not an attachment: {message_id}"));
        }
        if message.media_status != MediaStatus::Canceled {
            return Err(anyhow!("attachment is not retryable: {message_id}"));
        }
        if message.media_size.unwrap_or_default() == 0
            && message
                .media_url
                .as_deref()
                .and_then(|value| url::Url::parse(value).ok())
                .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
        {
            self.database
                .message_dao
                .update_media_status(message_id, MediaStatus::Pending)
                .await?;
            self.notify_messages_changed();
            let result = MessageAccess::new(self.state.clone())
                .complete_remote_image_from_url(&message, false)
                .await;
            if let Err(error) = result {
                self.database
                    .message_dao
                    .update_media_status(message_id, MediaStatus::Canceled)
                    .await?;
                self.notify_messages_changed();
                return Err(error);
            }
            return Ok(());
        }
        let stored_path = message
            .media_url
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| anyhow!("attachment has no local file: {message_id}"))?;
        let path = if std::path::Path::new(stored_path).is_absolute() {
            stored_path.to_string()
        } else {
            attachment_path(
                &account_data_directory(&self.profile.borrow().identity_number)?,
                &message,
            )?
            .to_string_lossy()
            .into_owned()
        };

        let cancellation = CancellationToken::new();
        {
            let mut transfers = self
                .attachment_downloads
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if transfers.contains_key(message_id) {
                return Err(anyhow!(
                    "attachment transfer is already active: {message_id}"
                ));
            }
            transfers.insert(message_id.to_string(), cancellation.clone());
        }
        self.database
            .message_dao
            .update_media_status(message_id, MediaStatus::Pending)
            .await?;
        self.notify_messages_changed();

        let result: Result<()> = async {
            let existing = message.content.as_deref().and_then(|content| {
                Base64::decode_vec(content)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<AttachmentMessage>(&bytes).ok())
                    .filter(|attachment| {
                        !attachment.attachment_id.trim().is_empty()
                            && attachment.created_at.is_some()
                    })
            });
            let attachment = match existing {
                Some(attachment) => attachment,
                None => {
                    let upload = tokio::select! {
                        result = self.app_service.attachment.upload(
                            std::path::Path::new(&path),
                            !message.category.starts_with("PLAIN_"),
                        ) => result?,
                        _ = cancellation.cancelled() => {
                            return Err(anyhow!("attachment upload canceled"));
                        }
                    };
                    AttachmentMessage {
                        key: upload.key,
                        digest: upload.digest,
                        attachment_id: upload.attachment_id,
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
                        created_at: Some(upload.created_at),
                        shareable: Some(true),
                    }
                }
            };
            if cancellation.is_cancelled() {
                return Err(anyhow!("attachment upload canceled"));
            }
            let content = Base64::encode_string(serde_json::to_string(&attachment)?.as_bytes());
            let completed = self
                .database
                .message_dao
                .complete_attachment_retry_if_pending(
                    message_id,
                    &AttachmentMessageUpdate {
                        status: MessageStatus::Sending,
                        content,
                        media_mime_type: attachment.mime_type.clone(),
                        media_size: attachment.size,
                        media_status: MediaStatus::Done,
                        media_width: attachment.width,
                        media_height: attachment.height,
                        media_digest: attachment.digest.clone(),
                        media_key: attachment.key.clone(),
                        media_waveform: attachment.waveform.clone(),
                        caption: attachment.caption.clone(),
                        name: attachment.name.clone(),
                        thumb_image: attachment.thumbnail.clone(),
                        media_duration: attachment.duration.map(|value| value.to_string()),
                    },
                )
                .await?;
            if !completed {
                return Err(anyhow!("attachment upload canceled"));
            }
            let conversation = self
                .database
                .conversation_dao
                .find_conversation_by_id(&message.conversation_id)
                .await?
                .ok_or_else(|| anyhow!("conversation not found: {}", message.conversation_id))?;
            let job = Job::create_sending_job(
                message_id,
                &message.conversation_id,
                None,
                None,
                false,
                false,
                conversation.expire_in,
            );
            self.app_service.job.add(&job).await?;
            Ok(())
        }
        .await;

        self.attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(message_id);
        if let Err(error) = result {
            self.database
                .message_dao
                .update_media_status(message_id, MediaStatus::Canceled)
                .await?;
            self.notify_messages_changed();
            if cancellation.is_cancelled() {
                return Ok(());
            }
            return Err(error);
        }
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn download_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<()> {
        let transcript_id = transcript_id.as_str();
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let parent = self
            .database
            .message_dao
            .find_message_by_id(&transcript_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("transcript message not found: {transcript_id}"))?;
        let transcript = self
            .database
            .transcript_message_dao
            .find(transcript_id, message_id)
            .await?
            .ok_or_else(|| anyhow!("transcript attachment not found: {message_id}"))?;
        let media_status = transcript.media_status.clone().unwrap_or_default();
        let message = transcript_attachment_message(&parent.conversation_id, &transcript)?;
        if !message.category.is_attachment() {
            return Err(anyhow!(
                "transcript message is not an attachment: {message_id}"
            ));
        }
        if media_status != MediaStatus::Canceled {
            return Err(anyhow!(
                "transcript attachment is not downloadable: {message_id}"
            ));
        }
        let content = transcript
            .content
            .as_deref()
            .ok_or_else(|| anyhow!("transcript attachment has no content"))?;
        let extra = AttachmentExtra {
            attachment_id: transcript_attachment_id(content)?,
            message_id: message_id.to_string(),
            shareable: None,
            created_at: transcript.media_created_at,
        };
        let download_key = transcript_download_key(transcript_id, message_id);
        let cancellation = CancellationToken::new();
        {
            let mut downloads = self
                .attachment_downloads
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if downloads.contains_key(&download_key) {
                return Err(anyhow!(
                    "transcript attachment download is already active: {message_id}"
                ));
            }
            downloads.insert(download_key.clone(), cancellation.clone());
        }

        self.set_attachment_progress(message_id, 0, 0);
        let progress = |completed, total| {
            self.set_attachment_progress(message_id, completed, total);
        };
        let result: Result<()> = async {
            self.database
                .transcript_message_dao
                .update_media_status(transcript_id, message_id, MediaStatus::Pending)
                .await?;
            self.notify_messages_changed();
            let downloaded = self
                .app_service
                .attachment
                .download_transcript_cancellable(&message, &extra, &cancellation, &progress)
                .await?;
            if cancellation.is_cancelled() {
                return Err(anyhow!("transcript attachment download canceled"));
            }
            let content = serde_json::to_string(&downloaded.attachment)?;
            let path = attachment_file_name(&downloaded.path)?;
            let completed = self
                .database
                .transcript_message_dao
                .complete_attachment_download_if_pending(
                    transcript_id,
                    message_id,
                    path,
                    downloaded.size,
                    downloaded.attachment.created_at,
                    &content,
                )
                .await?;
            if !completed {
                let _ = tokio::fs::remove_file(&downloaded.path).await;
                return Err(anyhow!("transcript attachment download canceled"));
            }
            Ok(())
        }
        .await;

        self.attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&download_key);
        self.remove_attachment_progress(message_id);
        if let Err(error) = result {
            self.database
                .transcript_message_dao
                .update_media_status(transcript_id, message_id, MediaStatus::Canceled)
                .await?;
            self.notify_messages_changed();
            if cancellation.is_cancelled() {
                return Ok(());
            }
            return Err(error);
        }
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn retry_transcript_attachment(&self, transcript_id: String) -> Result<()> {
        let transcript_id = transcript_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let parent = self
            .database
            .message_dao
            .find_message_by_id(&transcript_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("transcript message not found: {transcript_id}"))?;
        if parent.user_id != self.account_id {
            return Err(anyhow!("only outgoing transcripts can be retried"));
        }
        if !parent.category.is_transcript() {
            return Err(anyhow!("message is not a transcript: {transcript_id}"));
        }
        if parent.media_status == MediaStatus::Pending || parent.media_status == MediaStatus::Done {
            return Ok(());
        }

        let transcripts = self
            .database
            .transcript_message_dao
            .find_by_transcript_id(transcript_id)
            .await?;
        let attachments = transcripts
            .iter()
            .filter(|transcript| transcript.category.is_attachment())
            .collect::<Vec<_>>();
        if attachments.is_empty() {
            return Err(anyhow!("transcript has no attachments: {transcript_id}"));
        }

        self.database
            .message_dao
            .update_media_status(transcript_id, MediaStatus::Pending)
            .await?;
        self.notify_messages_changed();

        let result: Result<()> = async {
            let parent_prefix = parent.category.split_once('_').map(|(prefix, _)| prefix);
            let fresh_after = chrono::Utc::now() - chrono::Duration::days(1);
            for transcript in attachments {
                if transcript.media_status == Some(MediaStatus::Done) {
                    continue;
                }
                let stored_path = transcript
                    .media_url
                    .as_deref()
                    .filter(|path| !path.trim().is_empty())
                    .ok_or_else(|| {
                        anyhow!(
                            "transcript attachment has no local file: {}",
                            transcript.message_id
                        )
                    })?;
                let path = if std::path::Path::new(stored_path).is_absolute() {
                    stored_path.to_string()
                } else {
                    transcript_attachment_path(
                        &account_data_directory(&self.profile.borrow().identity_number)?,
                        &transcript_attachment_message(&parent.conversation_id, transcript)?,
                    )?
                    .to_string_lossy()
                    .into_owned()
                };
                let encrypted = !transcript.category.starts_with("PLAIN_");
                let credentials_valid = if encrypted {
                    transcript.media_key.is_some() && transcript.media_digest.is_some()
                } else {
                    transcript.media_key.is_none() && transcript.media_digest.is_none()
                };
                let same_encryption = transcript
                    .category
                    .split_once('_')
                    .map(|(prefix, _)| prefix)
                    == parent_prefix;
                let fresh = transcript
                    .media_created_at
                    .is_some_and(|created_at| created_at >= fresh_after);
                let existing_attachment_id = transcript
                    .content
                    .as_deref()
                    .and_then(|content| transcript_attachment_id(content).ok());

                let (attachment_id, created_at, key, digest) = if same_encryption
                    && credentials_valid
                    && fresh
                    && existing_attachment_id.is_some()
                {
                    (
                        existing_attachment_id.unwrap(),
                        transcript.media_created_at.unwrap(),
                        transcript.media_key.clone(),
                        transcript.media_digest.clone(),
                    )
                } else {
                    let upload = self
                        .app_service
                        .attachment
                        .upload(std::path::Path::new(&path), encrypted)
                        .await?;
                    (
                        upload.attachment_id,
                        upload.created_at,
                        upload.key.map(|value| Base64::encode_string(&value)),
                        upload.digest.map(|value| Base64::encode_string(&value)),
                    )
                };
                let content = serde_json::to_string(&AttachmentExtra {
                    attachment_id,
                    message_id: transcript.message_id.clone(),
                    shareable: Some(true),
                    created_at: Some(created_at),
                })?;
                self.database
                    .transcript_message_dao
                    .complete_attachment_upload(
                        transcript_id,
                        &transcript.message_id,
                        &content,
                        key.as_deref(),
                        digest.as_deref(),
                        created_at,
                    )
                    .await?;
            }
            self.database
                .message_dao
                .update_media_status(transcript_id, MediaStatus::Done)
                .await?;
            self.database
                .message_dao
                .update_message_status(transcript_id, MessageStatus::Sending)
                .await?;
            self.app_service.job.wake(sdk::SENDING_MESSAGE)?;
            Ok(())
        }
        .await;

        if let Err(error) = result {
            self.database
                .message_dao
                .update_media_status(transcript_id, MediaStatus::Canceled)
                .await?;
            self.notify_messages_changed();
            return Err(error);
        }
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn cancel_transcript_attachment(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<()> {
        let transcript_id = transcript_id.as_str();
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let download_key = transcript_download_key(transcript_id, message_id);
        let cancellation = self
            .attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&download_key)
            .cloned()
            .ok_or_else(|| anyhow!("transcript attachment download is not active: {message_id}"))?;
        cancellation.cancel();
        self.database
            .transcript_message_dao
            .update_media_status(transcript_id, message_id, MediaStatus::Canceled)
            .await?;
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn mark_transcript_audio_read(
        &self,
        transcript_id: String,
        message_id: String,
    ) -> Result<()> {
        let transcript_id = transcript_id.as_str();
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let transcript = self
            .database
            .transcript_message_dao
            .find(transcript_id, message_id)
            .await?
            .ok_or_else(|| anyhow!("transcript audio not found: {message_id}"))?;
        if !transcript.category.is_audio() {
            return Err(anyhow!("transcript message is not audio: {message_id}"));
        }
        match transcript.media_status.unwrap_or_default() {
            MediaStatus::Read => return Ok(()),
            MediaStatus::Done => {}
            _ => return Err(anyhow!("transcript audio is not downloaded: {message_id}")),
        }
        self.database
            .transcript_message_dao
            .update_media_status(transcript_id, message_id, MediaStatus::Read)
            .await?;
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn download_attachment(&self, message_id: String) -> Result<()> {
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?;
        if !message.category.is_attachment() {
            return Err(anyhow!("message is not an attachment: {message_id}"));
        }
        if message.media_status != MediaStatus::Canceled {
            return Err(anyhow!("attachment is not downloadable: {message_id}"));
        }
        let content = message
            .content
            .as_deref()
            .ok_or_else(|| anyhow!("attachment message has no content"))?;
        let extra: crate::core::model::AttachmentExtra = serde_json::from_str(content)?;
        let cancellation = CancellationToken::new();
        {
            let mut downloads = self
                .attachment_downloads
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if downloads.contains_key(message_id) {
                return Err(anyhow!(
                    "attachment download is already active: {message_id}"
                ));
            }
            downloads.insert(message_id.to_string(), cancellation.clone());
        }

        self.set_attachment_progress(message_id, 0, 0);
        let progress = |completed, total| {
            self.set_attachment_progress(message_id, completed, total);
        };
        let result: Result<()> = async {
            self.database
                .message_dao
                .update_media_status(message_id, MediaStatus::Pending)
                .await?;
            self.notify_messages_changed();
            let downloaded = self
                .app_service
                .attachment
                .download_cancellable(&message, &extra, &cancellation, &progress)
                .await?;
            if cancellation.is_cancelled() {
                return Err(anyhow!("attachment download canceled"));
            }
            let content = serde_json::to_string(&downloaded.attachment)?;
            let path = attachment_file_name(&downloaded.path)?;
            let completed = self
                .database
                .message_dao
                .complete_attachment_download_if_pending(
                    message_id,
                    path,
                    downloaded.size,
                    downloaded.status,
                    &content,
                )
                .await?;
            if !completed {
                let _ = tokio::fs::remove_file(&downloaded.path).await;
                return Err(anyhow!("attachment download canceled"));
            }
            self.database
                .message_dao
                .update_message_quote_if_need(&message.conversation_id, message_id)
                .await?;
            Ok(())
        }
        .await;

        self.attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(message_id);
        self.remove_attachment_progress(message_id);
        if let Err(error) = result {
            self.database
                .message_dao
                .update_media_status(message_id, MediaStatus::Canceled)
                .await?;
            self.notify_messages_changed();
            if cancellation.is_cancelled() {
                return Ok(());
            }
            return Err(error);
        }
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn cancel_attachment(&self, message_id: String) -> Result<()> {
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let cancellation = self
            .attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(message_id)
            .cloned()
            .ok_or_else(|| anyhow!("attachment download is not active: {message_id}"))?;
        cancellation.cancel();
        self.database
            .message_dao
            .update_media_status(message_id, MediaStatus::Canceled)
            .await?;
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn mark_audio_read(&self, message_id: String) -> Result<()> {
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?;
        if !message.category.is_audio() {
            return Err(anyhow!("message is not audio: {message_id}"));
        }
        if message.media_status == MediaStatus::Read {
            return Ok(());
        }
        if message.media_status != MediaStatus::Done {
            return Err(anyhow!("audio message is not downloaded: {message_id}"));
        }
        self.database
            .message_dao
            .update_media_status(message_id, MediaStatus::Read)
            .await?;
        self.notify_messages_changed();
        Ok(())
    }
}
