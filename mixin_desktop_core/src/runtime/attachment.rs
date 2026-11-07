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

#[path = "attachment_export.rs"]
mod export;
#[path = "attachment_recovery.rs"]
mod recovery;
#[path = "attachment_text.rs"]
mod text;

pub struct AttachmentAccess {
    state: Arc<AccountState>,
}

impl AttachmentAccess {
    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }

    /// Restarts the persisted transfer direction. Synced outgoing messages download
    /// like received messages; only unsent outgoing messages retry their upload.
    pub async fn resume_attachment(
        &self,
        message_id: String,
        transcript_id: Option<String>,
    ) -> Result<()> {
        self.ensure_active()?;
        let parent_id = transcript_id.as_ref().unwrap_or(&message_id);
        let parent = self
            .database
            .message_dao
            .find_message_by_id(parent_id)
            .await?
            .ok_or_else(|| anyhow!("message not found: {parent_id}"))?;
        if needs_upload_retry(&parent, &self.account_id) {
            if parent.category.is_transcript() {
                self.retry_transcript_attachment(parent.message_id).await
            } else {
                self.retry_attachment(message_id).await
            }
        } else if let Some(transcript_id) = transcript_id {
            self.download_transcript_attachment(transcript_id, message_id)
                .await
        } else {
            self.download_attachment(message_id).await
        }
    }

    async fn refresh_transcript_media_status(&self, transcript_id: &str) -> Result<()> {
        let transcripts = self
            .database
            .transcript_message_dao
            .find_by_transcript_id(transcript_id)
            .await?;
        let attachments = transcripts
            .iter()
            .filter(|message| message.category.is_attachment())
            .collect::<Vec<_>>();
        if attachments.is_empty() {
            return Ok(());
        }
        let status = if attachments
            .iter()
            .all(|message| message.media_status == Some(MediaStatus::Done))
        {
            MediaStatus::Done
        } else {
            MediaStatus::Canceled
        };
        self.database
            .message_dao
            .update_media_status(transcript_id, status)
            .await?;
        Ok(())
    }
}

impl Deref for AttachmentAccess {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl AttachmentAccess {
    pub async fn save_attachment_as(&self, message_id: String, destination: String) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let directory = account_data_directory(&self.profile.borrow().identity_number)?;
        let (source, _) = export::source(&self.database, &directory, &message_id, None).await?;
        export::save(&source, std::path::Path::new(&destination)).await
    }

    pub async fn save_transcript_attachment_as(
        &self,
        transcript_id: String,
        message_id: String,
        destination: String,
    ) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let directory = account_data_directory(&self.profile.borrow().identity_number)?;
        let (source, _) = export::source(
            &self.database,
            &directory,
            &message_id,
            Some(&transcript_id),
        )
        .await?;
        export::save(&source, std::path::Path::new(&destination)).await
    }

    /// Returns a decoded and normalized image from the current account attachment.
    pub async fn attachment_image_data_uri(
        &self,
        message_id: String,
        transcript_id: Option<String>,
    ) -> Result<String> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let directory = account_data_directory(&self.profile.borrow().identity_number)?;
        let (source, image) = export::source(
            &self.database,
            &directory,
            &message_id,
            transcript_id.as_deref(),
        )
        .await?;
        if !image {
            return Err(anyhow!("attachment is not an image: {message_id}"));
        }
        let bytes = self
            .app_service
            .attachment
            .read_account_file(&source, export::MAX_IMAGE_BYTES)
            .await?;
        tokio::task::spawn_blocking(move || export::image_data_uri(bytes)).await?
    }

    pub async fn retry_attachment(&self, message_id: String) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.retry_attachment_locked(&message_id).await
    }

    pub(super) async fn retry_attachment_locked(&self, message_id: &str) -> Result<()> {
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?;
        if message.user_id != self.account_id {
            return Err(anyhow!("only outgoing attachments can be retried"));
        }
        MessageAccess::new(self.state.clone())
            .ensure_can_send(&message.conversation_id)
            .await?;
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
            return MessageAccess::new(self.state.clone())
                .complete_remote_image_from_url(&message, false)
                .await;
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
        self.set_attachment_progress(message_id, 0, 0);
        let progress_state = self.state.clone();
        let progress_message_id = message_id.to_string();
        let progress = Arc::new(move |completed, total| {
            progress_state.set_attachment_progress(&progress_message_id, completed, total);
        });

        let result: Result<()> = async {
            self.database
                .message_dao
                .update_media_status(message_id, MediaStatus::Pending)
                .await?;
            self.notify_messages_changed();
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
                            Some(&cancellation),
                            Some(progress.clone()),
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
            let completed = self
                .database
                .message_dao
                .complete_pending_attachment(
                    message_id,
                    stored_path,
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
                    &job,
                )
                .await?;
            if !completed {
                return Err(anyhow!("attachment upload canceled"));
            }
            self.app_service.job.wake(&job.action)?;
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
                .cancel_pending_transfer(message_id)
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
        let directory = account_data_directory(&self.profile.borrow().identity_number)?;
        if !recovery::download_required(
            &self.database,
            &directory,
            message_id,
            Some(transcript_id),
            &message.category,
            media_status,
        )
        .await?
        {
            return Ok(());
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

        self.set_transcript_attachment_progress(transcript_id, message_id, 0, 0);
        let progress = |completed, total| {
            self.set_transcript_attachment_progress(transcript_id, message_id, completed, total);
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
                let _ = crate::fs::remove_file(&downloaded.path).await;
                return Err(anyhow!("transcript attachment download canceled"));
            }
            Ok(())
        }
        .await;

        self.attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&download_key);
        self.remove_transcript_attachment_progress(transcript_id, message_id);
        if let Err(error) = result {
            self.database
                .transcript_message_dao
                .cancel_pending_transfer(transcript_id, message_id)
                .await?;
            self.refresh_transcript_media_status(transcript_id).await?;
            self.notify_messages_changed();
            if cancellation.is_cancelled() {
                return Ok(());
            }
            return Err(error);
        }
        self.refresh_transcript_media_status(transcript_id).await?;
        self.notify_messages_changed();
        Ok(())
    }

    pub async fn retry_transcript_attachment(&self, transcript_id: String) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.retry_transcript_attachment_locked(&transcript_id)
            .await
    }

    pub(super) async fn retry_transcript_attachment_locked(
        &self,
        transcript_id: &str,
    ) -> Result<()> {
        let parent = self
            .database
            .message_dao
            .find_message_by_id(&transcript_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("transcript message not found: {transcript_id}"))?;
        if parent.user_id != self.account_id {
            return Err(anyhow!("only outgoing transcripts can be retried"));
        }
        MessageAccess::new(self.state.clone())
            .ensure_can_send(&parent.conversation_id)
            .await?;
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
            let parent_prefix = parent
                .category
                .split_once('_')
                .map(|(prefix, _)| prefix)
                .ok_or_else(|| anyhow!("invalid transcript category"))?;
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
                let (target_category, reusable) =
                    transcript_upload_plan(transcript, parent_prefix, chrono::Utc::now())?;
                let encrypted = !target_category.starts_with("PLAIN_");
                let existing_attachment_id = transcript
                    .content
                    .as_deref()
                    .and_then(|content| transcript_attachment_id(content).ok());

                let (attachment_id, created_at, key, digest) = if reusable {
                    let attachment_id = match existing_attachment_id {
                        Some(attachment_id) => attachment_id,
                        None => unreachable!("existing attachment ID was checked"),
                    };
                    self.database
                        .transcript_message_dao
                        .update_media_status(
                            transcript_id,
                            &transcript.message_id,
                            MediaStatus::Pending,
                        )
                        .await?;
                    (
                        attachment_id,
                        transcript.media_created_at.unwrap(),
                        transcript.media_key.clone(),
                        transcript.media_digest.clone(),
                    )
                } else {
                    let download_key =
                        transcript_download_key(transcript_id, &transcript.message_id);
                    let cancellation = CancellationToken::new();
                    {
                        let mut transfers = self
                            .attachment_downloads
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        if transfers.contains_key(&download_key) {
                            return Err(anyhow!(
                                "transcript attachment transfer is already active: {}",
                                transcript.message_id
                            ));
                        }
                        transfers.insert(download_key.clone(), cancellation.clone());
                    }
                    self.database
                        .transcript_message_dao
                        .update_media_status(
                            transcript_id,
                            &transcript.message_id,
                            MediaStatus::Pending,
                        )
                        .await?;
                    self.set_transcript_attachment_progress(
                        transcript_id,
                        &transcript.message_id,
                        0,
                        0,
                    );
                    self.notify_messages_changed();
                    let progress_state = self.state.clone();
                    let progress_message_id = transcript.message_id.clone();
                    let progress_transcript_id = transcript_id.to_owned();
                    let progress = Arc::new(move |completed, total| {
                        progress_state.set_transcript_attachment_progress(
                            &progress_transcript_id,
                            &progress_message_id,
                            completed,
                            total,
                        );
                    });
                    let upload = self
                        .app_service
                        .attachment
                        .upload(
                            std::path::Path::new(&path),
                            encrypted,
                            Some(&cancellation),
                            Some(progress),
                        )
                        .await;
                    self.attachment_downloads
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .remove(&download_key);
                    self.remove_transcript_attachment_progress(
                        transcript_id,
                        &transcript.message_id,
                    );
                    let upload = match upload {
                        Ok(upload) => upload,
                        Err(error) => {
                            self.database
                                .transcript_message_dao
                                .cancel_pending_transfer(transcript_id, &transcript.message_id)
                                .await?;
                            return Err(error);
                        }
                    };
                    (
                        upload.attachment_id,
                        upload.created_at,
                        upload.key.map(|value| Base64::encode_string(&value)),
                        upload.digest.map(|value| Base64::encode_string(&value)),
                    )
                };
                let completed = self
                    .database
                    .transcript_message_dao
                    .complete_attachment_upload_if_pending(
                        transcript_id,
                        &transcript.message_id,
                        &target_category,
                        &attachment_id,
                        key.as_deref(),
                        digest.as_deref(),
                        created_at,
                    )
                    .await?;
                if !completed {
                    return Err(anyhow!("transcript attachment upload canceled"));
                }
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
                .cancel_pending_transfer(transcript_id)
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
            .cloned();
        let Some(cancellation) = cancellation else {
            return Ok(());
        };
        cancellation.cancel();
        self.database
            .transcript_message_dao
            .cancel_pending_transfer(transcript_id, message_id)
            .await?;
        self.refresh_transcript_media_status(transcript_id).await?;
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
        let directory = account_data_directory(&self.profile.borrow().identity_number)?;
        if !recovery::download_required(
            &self.database,
            &directory,
            message_id,
            None,
            &message.category,
            message.media_status.clone(),
        )
        .await?
        {
            return Ok(());
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
                let _ = crate::fs::remove_file(&downloaded.path).await;
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
                .cancel_pending_transfer(message_id)
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
            .cloned();
        let Some(cancellation) = cancellation else {
            return Ok(());
        };
        cancellation.cancel();
        self.database
            .message_dao
            .cancel_pending_transfer(message_id)
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

fn needs_upload_retry(message: &crate::db::mixin::message::Message, account_id: &str) -> bool {
    message.user_id == account_id
        && matches!(
            message.status,
            MessageStatus::Sending | MessageStatus::Failed
        )
}

fn transcript_upload_plan(
    transcript: &crate::db::mixin::transcript_message::TranscriptMessage,
    target_prefix: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(String, bool)> {
    let target_category = super::forward_category(&transcript.category, target_prefix)?;
    let plain = transcript.category.starts_with("PLAIN_");
    let credentials_valid = if plain {
        transcript.media_key.is_none() && transcript.media_digest.is_none()
    } else {
        [
            (transcript.media_key.as_deref(), 64),
            (transcript.media_digest.as_deref(), 32),
        ]
        .into_iter()
        .all(|(value, length)| {
            value.is_some_and(|value| {
                Base64::decode_vec(value).is_ok_and(|bytes| bytes.len() == length)
            })
        })
    };
    let fresh = transcript.media_created_at.is_some_and(|created_at| {
        created_at <= now && created_at >= now - chrono::Duration::days(1)
    });
    let reusable = transcript.category == target_category
        && credentials_valid
        && fresh
        && transcript
            .content
            .as_deref()
            .is_some_and(|content| transcript_attachment_id(content).is_ok());
    Ok((target_category, reusable))
}

#[cfg(test)]
mod transcript_upload_tests {
    use super::*;
    use crate::db::{mixin::transcript_message::TranscriptMessage, MixinDatabase};

    #[tokio::test]
    async fn persisted_unsent_outgoing_transfer_retries_upload_but_synced_or_received_transfer_downloads(
    ) {
        use crate::db::mixin::message::Message;
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        crate::db::query("INSERT INTO conversations (conversation_id, created_at, status, category) VALUES ('conversation', ?, 2, 'CONTACT')")
            .bind(chrono::Utc::now()).execute(&database.message_dao.0).await.unwrap();
        for (id, user, status, expected) in [
            ("sending", "me", MessageStatus::Sending, true),
            ("failed", "me", MessageStatus::Failed, true),
            ("synced", "me", MessageStatus::Sent, false),
            ("received", "peer", MessageStatus::Sending, false),
            ("unknown", "me", MessageStatus::Unknown, false),
        ] {
            database
                .message_dao
                .insert_message(&Message {
                    message_id: id.into(),
                    conversation_id: "conversation".into(),
                    user_id: user.into(),
                    status,
                    category: "PLAIN_DATA".into(),
                    media_status: MediaStatus::Canceled,
                    media_url: Some("/local/source.txt".into()),
                    ..Default::default()
                })
                .await
                .unwrap();
            let stored = database
                .message_dao
                .find_message_by_id(&id.to_owned())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(needs_upload_retry(&stored, "me"), expected, "{id}");
        }
        database.close().await;
    }

    #[tokio::test]
    async fn reopening_interrupted_transcript_preserves_completed_children_and_original_send_job() {
        use crate::db::mixin::message::Message;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mixin.db");
        let database = MixinDatabase::connect_at(&path).await.unwrap();
        let now = chrono::Utc::now();
        crate::db::query("INSERT INTO conversations (conversation_id, created_at, status, category) VALUES ('conversation', ?, 2, 'CONTACT')")
            .bind(now).execute(&database.message_dao.0).await.unwrap();
        let parent = Message {
            message_id: "root".into(),
            conversation_id: "conversation".into(),
            user_id: "me".into(),
            category: "PLAIN_TRANSCRIPT".into(),
            content: Some("[]".into()),
            status: MessageStatus::Sending,
            media_status: MediaStatus::Pending,
            ..Default::default()
        };
        let source = directory.path().join("source.txt");
        std::fs::write(&source, b"preserved source bytes").unwrap();
        let children: Vec<TranscriptMessage> = serde_json::from_value(serde_json::json!([
            {"transcript_id":"root", "message_id":"completed", "category":"PLAIN_DATA", "created_at":now.to_rfc3339(),
             "content":"completed-upload", "media_status":"DONE", "media_url":source.to_str(), "media_created_at":now.to_rfc3339()},
            {"transcript_id":"root", "message_id":"interrupted", "category":"PLAIN_DATA", "created_at":now.to_rfc3339(),
             "content":"reusable-upload", "media_status":"PENDING", "media_url":source.to_str(), "media_created_at":now.to_rfc3339()}
        ])).unwrap();
        let job = Job::create_sending_job("root", "conversation", None, None, false, false, 0);
        database
            .message_dao
            .insert_outgoing_message_with_transcripts(&parent, &job, &children)
            .await
            .unwrap();
        for (id, user, status, category, media) in [
            (
                "ready",
                "me",
                MessageStatus::Sending,
                "SIGNAL_TRANSCRIPT",
                MediaStatus::Done,
            ),
            (
                "sent",
                "me",
                MessageStatus::Sent,
                "SIGNAL_TRANSCRIPT",
                MediaStatus::Pending,
            ),
            (
                "peer",
                "peer",
                MessageStatus::Sending,
                "SIGNAL_TRANSCRIPT",
                MediaStatus::Pending,
            ),
            (
                "audio",
                "me",
                MessageStatus::Sending,
                "SIGNAL_AUDIO",
                MediaStatus::Pending,
            ),
        ] {
            database
                .message_dao
                .insert_message(&Message {
                    message_id: id.into(),
                    user_id: user.into(),
                    status,
                    category: category.into(),
                    media_status: media,
                    ..parent.clone()
                })
                .await
                .unwrap();
        }
        database.close().await;
        let database = MixinDatabase::connect_at(&path).await.unwrap();
        database
            .transcript_message_dao
            .recover_interrupted_uploads("me")
            .await
            .unwrap();
        let recovered = database
            .message_dao
            .find_message_by_id(&"root".into())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.media_status, MediaStatus::Canceled);
        assert_eq!(recovered.status, MessageStatus::Sending);
        assert_eq!(
            database
                .message_dao
                .find_message_by_id(&"ready".into())
                .await
                .unwrap()
                .unwrap()
                .media_status,
            MediaStatus::Done
        );
        for id in ["sent", "peer", "audio"] {
            assert_eq!(
                database
                    .message_dao
                    .find_message_by_id(&id.to_string())
                    .await
                    .unwrap()
                    .unwrap()
                    .media_status,
                MediaStatus::Pending
            );
        }
        let completed = database
            .transcript_message_dao
            .find("root", "completed")
            .await
            .unwrap()
            .unwrap();
        let interrupted = database
            .transcript_message_dao
            .find("root", "interrupted")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(completed.media_status, Some(MediaStatus::Done));
        assert_eq!(completed.content.as_deref(), Some("completed-upload"));
        assert_eq!(interrupted.media_status, Some(MediaStatus::Canceled));
        assert!(
            transcript_upload_plan(&interrupted, "PLAIN", now)
                .unwrap()
                .1
        );
        assert_eq!(std::fs::read(source).unwrap(), b"preserved source bytes");
        let jobs = database.job_dao.sending_jobs().await.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, job.job_id);
        // Resume the uploader's persisted completion using the recovered child and original parent/job IDs.
        database
            .transcript_message_dao
            .complete_attachment_upload(
                "root",
                "interrupted",
                "PLAIN_DATA",
                "reusable-upload",
                None,
                None,
                now,
            )
            .await
            .unwrap();
        database
            .message_dao
            .update_media_status("root", MediaStatus::Done)
            .await
            .unwrap();
        database.close().await;
        let database = MixinDatabase::connect_at(path).await.unwrap();
        database
            .transcript_message_dao
            .recover_interrupted_uploads("me")
            .await
            .unwrap();
        let parent = database
            .message_dao
            .find_sending_message("root")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(parent.media_status, MediaStatus::Done);
        assert!(database
            .transcript_message_dao
            .find_by_transcript_id("root")
            .await
            .unwrap()
            .iter()
            .all(|row| row.media_status == Some(MediaStatus::Done)));
        assert_eq!(
            database.job_dao.sending_jobs().await.unwrap()[0].job_id,
            job.job_id
        );
    }

    #[tokio::test]
    async fn transcript_cross_encryption_requires_upload_and_persists_target_credentials() {
        let now = chrono::DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let mut transcript: TranscriptMessage = serde_json::from_value(serde_json::json!({
            "transcript_id":"root", "message_id":"file", "category":"SIGNAL_DATA",
            "created_at":now.to_rfc3339(), "content":"old-id", "media_url":"/source/file.txt",
            "media_key":Base64::encode_string(&[1;64]), "media_digest":Base64::encode_string(&[2;32]),
            "media_created_at":now.to_rfc3339(), "media_status":"CANCELED"
        })).unwrap();
        assert_eq!(
            transcript_upload_plan(&transcript, "SIGNAL", now).unwrap(),
            ("SIGNAL_DATA".into(), true)
        );
        assert_eq!(
            transcript_upload_plan(&transcript, "PLAIN", now).unwrap(),
            ("PLAIN_DATA".into(), false)
        );
        assert_eq!(
            transcript_upload_plan(&transcript, "ENCRYPTED", now).unwrap(),
            ("ENCRYPTED_DATA".into(), false)
        );
        transcript.media_created_at = Some(now - chrono::Duration::hours(25));
        assert!(
            !transcript_upload_plan(&transcript, "SIGNAL", now)
                .unwrap()
                .1
        );
        transcript.media_created_at = Some(now + chrono::Duration::seconds(1));
        assert!(
            !transcript_upload_plan(&transcript, "SIGNAL", now)
                .unwrap()
                .1
        );
        transcript.media_created_at = Some(now);
        transcript.media_key = Some("invalid-key".into());
        assert!(
            !transcript_upload_plan(&transcript, "SIGNAL", now)
                .unwrap()
                .1
        );

        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        database
            .transcript_message_dao
            .insert_all(&[transcript])
            .await
            .unwrap();
        database
            .transcript_message_dao
            .complete_attachment_upload("root", "file", "PLAIN_DATA", "new-id", None, None, now)
            .await
            .unwrap();
        let stored = database
            .transcript_message_dao
            .find("root", "file")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.category, "PLAIN_DATA");
        assert_eq!(stored.content.as_deref(), Some("new-id"));
        assert_eq!(stored.media_key, None);
        assert_eq!(stored.media_digest, None);
        assert_eq!(stored.media_status, Some(MediaStatus::Done));
        assert_eq!(stored.media_url.as_deref(), Some("/source/file.txt"));
        assert!(transcript_upload_plan(&stored, "PLAIN", now).unwrap().1);
    }
}
