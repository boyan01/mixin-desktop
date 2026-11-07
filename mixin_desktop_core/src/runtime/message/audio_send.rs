//! Account-owned message audio send operations.

use super::{
    super::{model, MAX_AUDIO_DURATION_MILLIS, MAX_AUDIO_FILE_SIZE, MAX_AUDIO_WAVEFORM_SAMPLES},
    MessageAccess,
};
use crate::{
    core::attachment::attachment_file_name,
    db::mixin::{
        job::Job,
        message::{AttachmentMessageUpdate, MediaStatus, Message},
    },
};
use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use log::warn;
use sdk::{AttachmentMessage, MessageStatus};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

impl MessageAccess {
    pub async fn send_audio(
        &self,
        conversation_id: String,
        path: String,
        duration_millis: i64,
        waveform: Vec<u8>,
        quote_message_id: Option<String>,
    ) -> Result<String> {
        let receipt = self
            .send_audio_receipt(
                conversation_id,
                path,
                duration_millis,
                waveform,
                quote_message_id,
            )
            .await?;
        if let Some(error) = receipt.upload_error {
            return Err(anyhow!(error));
        }
        Ok(receipt.message_id)
    }

    pub async fn send_audio_receipt(
        &self,
        conversation_id: String,
        path: String,
        duration_millis: i64,
        waveform: Vec<u8>,
        quote_message_id: Option<String>,
    ) -> Result<model::SendReceipt> {
        let quote_message_id = quote_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        let path = path.as_str();
        let waveform = waveform.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.ensure_can_send(conversation_id).await?;
        if !(1..=MAX_AUDIO_DURATION_MILLIS).contains(&duration_millis) {
            return Err(anyhow!(
                "audio duration must be between 1 and 60000 milliseconds"
            ));
        }
        if waveform.is_empty() || waveform.len() > MAX_AUDIO_WAVEFORM_SAMPLES {
            return Err(anyhow!(
                "audio waveform must contain between 1 and 1024 samples"
            ));
        }
        let source = crate::fs::canonicalize(path)
            .await
            .with_context(|| format!("resolve recorded audio {path}"))?;
        let metadata = crate::fs::metadata(&source).await?;
        if !metadata.is_file() || metadata.is_empty() {
            return Err(anyhow!("recorded audio is empty or is not a file"));
        }
        if metadata.len() > MAX_AUDIO_FILE_SIZE {
            return Err(anyhow!("recorded audio exceeds the 64 MiB size limit"));
        }

        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        let text_category = self.text_category(conversation.owner_id.as_deref()).await?;
        let prefix = text_category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid message category: {text_category}"))?;
        let category = format!("{prefix}_AUDIO");
        let quote_content = match quote_message_id {
            Some(message_id) => self
                .database
                .message_dao
                .find_quote_message_by_id(message_id)
                .await?
                .map(|message| serde_json::to_string(&message))
                .transpose()?,
            None => None,
        };
        if quote_message_id.is_some() && quote_content.is_none() {
            return Err(anyhow!("quote message not found"));
        }

        let message_id = Uuid::new_v4().to_string();
        let file_message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            category: category.clone(),
            media_mime_type: Some("audio/ogg".to_string()),
            ..Message::default()
        };
        let (local_path, actual_size) = self
            .app_service
            .attachment
            .import_local(&source, &file_message)
            .await?;
        let encoded_waveform = Base64::encode_string(waveform);
        let message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category,
            content: Some(String::new()),
            media_url: Some(attachment_file_name(&local_path)?.to_string()),
            media_mime_type: Some("audio/ogg".to_string()),
            media_size: Some(actual_size),
            media_duration: duration_millis.to_string(),
            media_status: MediaStatus::Pending,
            media_waveform: Some(encoded_waveform.clone()),
            quote_message_id: quote_message_id.map(str::to_string),
            quote_content,
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        self.database
            .message_dao
            .insert_pending_outgoing_message(&message)
            .await
            .map_err(crate::CoreError::from)?;
        self.notify_conversation_changed(conversation_id);
        let accepted_id = message_id.clone();
        let upload_result: Result<String> = async {
            let cancellation = CancellationToken::new();
            self.attachment_downloads
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(message_id.clone(), cancellation.clone());
            self.set_attachment_progress(&message_id, 0, 0);
            let progress_state = self.state.clone();
            let progress_message_id = message_id.clone();
            let progress = Arc::new(move |completed, total| {
                progress_state.set_attachment_progress(&progress_message_id, completed, total);
            });
            let upload = tokio::select! {
                result = self.app_service.attachment.upload(
                    &local_path,
                    prefix != "PLAIN",
                    Some(&cancellation),
                    Some(progress.clone()),
                ) => result,
                _ = cancellation.cancelled() => Err(anyhow!("attachment upload canceled")),
            };
            self.attachment_downloads
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&message_id);
            self.remove_attachment_progress(&message_id);
            let upload = match upload {
                Ok(upload) => upload,
                Err(error) => {
                    self.database
                        .message_dao
                        .update_media_status(&message_id, MediaStatus::Canceled)
                        .await?;
                    self.notify_conversation_changed(conversation_id);
                    if cancellation.is_cancelled() {
                        return Ok(message_id);
                    }
                    return Err(anyhow!("attachment_upload_failed:{error}"));
                }
            };
            let attachment = AttachmentMessage {
                key: upload.key.clone(),
                digest: upload.digest.clone(),
                attachment_id: upload.attachment_id,
                mime_type: "audio/ogg".to_string(),
                size: actual_size,
                name: None,
                width: None,
                height: None,
                thumbnail: None,
                duration: Some(duration_millis),
                waveform: Some(waveform.to_vec()),
                caption: None,
                created_at: Some(upload.created_at),
                shareable: Some(true),
            };
            let content = Base64::encode_string(serde_json::to_string(&attachment)?.as_bytes());
            let job = Job::create_sending_job(
                &message_id,
                conversation_id,
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
                    &message_id,
                    &local_path.to_string_lossy(),
                    &AttachmentMessageUpdate {
                        status: MessageStatus::Sending,
                        content,
                        media_mime_type: "audio/ogg".to_string(),
                        media_size: actual_size,
                        media_status: MediaStatus::Done,
                        media_width: None,
                        media_height: None,
                        media_digest: upload.digest,
                        media_key: upload.key,
                        media_waveform: Some(waveform.to_vec()),
                        caption: None,
                        name: None,
                        thumb_image: None,
                        media_duration: Some(duration_millis.to_string()),
                    },
                    &job,
                )
                .await?;
            if !completed {
                return Ok(message_id);
            }
            self.app_service.job.wake(&job.action)?;
            self.notify_conversation_changed(conversation_id);
            Ok(message_id)
        }
        .await;
        let upload_error = upload_result.err().map(|error| error.to_string());
        if upload_error.is_some() {
            if let Err(error) = self
                .database
                .message_dao
                .cancel_pending_attachment(&accepted_id)
                .await
            {
                warn!("Marking an accepted attachment as retryable failed: {error}");
            }
            self.notify_conversation_changed(conversation_id);
        }
        Ok(model::SendReceipt {
            message_id: accepted_id,
            upload_error,
        })
    }
}
