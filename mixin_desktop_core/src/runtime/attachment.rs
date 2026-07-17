use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use sdk::message_category::MessageCategory as _;
use tokio_util::sync::CancellationToken;

use crate::core::message::decrypt::{transcript_attachment_id, transcript_attachment_message};
use crate::core::model::AttachmentExtra;
use crate::db::mixin::message::MediaStatus;

use super::{transcript_download_key, AccountState};

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

        let result: Result<()> = async {
            self.database
                .transcript_message_dao
                .update_media_status(transcript_id, message_id, MediaStatus::Pending)
                .await?;
            self.notify_conversation_changed();
            let downloaded = self
                .app_service
                .attachment
                .download_transcript_cancellable(&message, &extra, &cancellation)
                .await?;
            if cancellation.is_cancelled() {
                return Err(anyhow!("transcript attachment download canceled"));
            }
            let content = serde_json::to_string(&downloaded.attachment)?;
            let path = downloaded
                .path
                .to_str()
                .ok_or_else(|| anyhow!("attachment path is not valid UTF-8"))?;
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
        if let Err(error) = result {
            self.database
                .transcript_message_dao
                .update_media_status(transcript_id, message_id, MediaStatus::Canceled)
                .await?;
            self.notify_conversation_changed();
            if cancellation.is_cancelled() {
                return Ok(());
            }
            return Err(error);
        }
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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

        let result: Result<()> = async {
            self.database
                .message_dao
                .update_media_status(message_id, MediaStatus::Pending)
                .await?;
            self.notify_conversation_changed();
            let downloaded = self
                .app_service
                .attachment
                .download_cancellable(&message, &extra, &cancellation)
                .await?;
            if cancellation.is_cancelled() {
                return Err(anyhow!("attachment download canceled"));
            }
            let content = serde_json::to_string(&downloaded.attachment)?;
            let path = downloaded
                .path
                .to_str()
                .ok_or_else(|| anyhow!("attachment path is not valid UTF-8"))?;
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
            self.notify_conversation_changed();
            if cancellation.is_cancelled() {
                return Ok(());
            }
            return Err(error);
        }
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
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
        self.notify_conversation_changed();
        Ok(())
    }
}
