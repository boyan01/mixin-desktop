//! Account-owned message attachment send operations.

use super::{super::model, MessageAccess};
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
    #[allow(clippy::too_many_arguments)]
    pub async fn send_attachment(
        &self,
        conversation_id: String,
        path: String,
        kind: String,
        mime_type: String,
        name: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
        duration_millis: Option<i64>,
        thumbnail: Option<String>,
        caption: Option<String>,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String> {
        let receipt = self
            .send_attachment_receipt(
                conversation_id,
                path,
                kind,
                mime_type,
                name,
                width,
                height,
                duration_millis,
                thumbnail,
                caption,
                quote_message_id,
                silent,
            )
            .await?;
        if let Some(error) = receipt.upload_error {
            return Err(anyhow!(error));
        }
        Ok(receipt.message_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_attachment_receipt(
        &self,
        conversation_id: String,
        path: String,
        kind: String,
        mime_type: String,
        name: Option<String>,
        width: Option<i32>,
        height: Option<i32>,
        duration_millis: Option<i64>,
        thumbnail: Option<String>,
        caption: Option<String>,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<model::SendReceipt> {
        let quote_message_id = quote_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        let path = path.as_str();
        let kind = kind.trim().to_ascii_uppercase();
        let mime_type = mime_type.trim();
        let name = name.filter(|value| !value.trim().is_empty());
        let caption = caption.filter(|value| !value.trim().is_empty());
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.ensure_can_send(conversation_id).await?;

        if !matches!(kind.as_str(), "IMAGE" | "VIDEO" | "DATA") {
            return Err(anyhow!("unsupported attachment kind: {kind}"));
        }
        if mime_type.is_empty() {
            return Err(anyhow!("attachment MIME type is required"));
        }
        if matches!(kind.as_str(), "IMAGE" | "VIDEO")
            && (width.is_none_or(|value| value <= 0) || height.is_none_or(|value| value <= 0))
        {
            return Err(anyhow!("media dimensions must be positive"));
        }
        if kind == "VIDEO" && duration_millis.is_none_or(|value| value <= 0) {
            return Err(anyhow!("video duration must be positive"));
        }
        if kind == "DATA" && name.is_none() {
            return Err(anyhow!("file name is required"));
        }

        let source = crate::fs::canonicalize(path)
            .await
            .with_context(|| format!("resolve attachment {path}"))?;
        let metadata = crate::fs::metadata(&source).await?;
        if !metadata.is_file() || metadata.is_empty() {
            return Err(anyhow!("attachment is empty or is not a file"));
        }
        let size = i64::try_from(metadata.len()).context("attachment is too large")?;

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
        let category = format!("{prefix}_{kind}");
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
            media_mime_type: Some(mime_type.to_string()),
            name: name.clone(),
            ..Message::default()
        };
        let (local_path, actual_size) = self
            .app_service
            .attachment
            .import_local(&source, &file_message)
            .await?;
        if actual_size != size {
            warn!("attachment size changed while importing {path}: {size} -> {actual_size}");
        }
        let message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category,
            content: Some(String::new()),
            media_url: Some(attachment_file_name(&local_path)?.to_string()),
            media_mime_type: Some(mime_type.to_string()),
            media_size: Some(actual_size),
            media_width: width,
            media_height: height,
            media_duration: duration_millis
                .map(|value| value.to_string())
                .unwrap_or_default(),
            thumb_image: thumbnail.clone(),
            media_status: MediaStatus::Pending,
            quote_message_id: quote_message_id.map(str::to_string),
            quote_content,
            caption: caption.clone(),
            name: name.clone(),
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
                mime_type: mime_type.to_string(),
                size: actual_size,
                name: name.clone(),
                width,
                height,
                thumbnail: thumbnail.clone(),
                duration: duration_millis,
                waveform: None,
                caption: caption.clone(),
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
                silent,
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
                        media_mime_type: mime_type.to_string(),
                        media_size: actual_size,
                        media_status: MediaStatus::Done,
                        media_width: width,
                        media_height: height,
                        media_digest: upload.digest,
                        media_key: upload.key,
                        media_waveform: None,
                        caption,
                        name,
                        thumb_image: thumbnail,
                        media_duration: duration_millis.map(|value| value.to_string()),
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
