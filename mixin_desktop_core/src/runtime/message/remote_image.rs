//! Account-owned message remote image operations.

use super::MessageAccess;
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
use std::{io::Cursor, path::Path, sync::Arc};
#[cfg(target_arch = "wasm32")]
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_REMOTE_IMAGE_BYTES: usize = 32 * 1024 * 1024;
const MAX_REMOTE_IMAGE_PIXELS: u64 = 32 * 1024 * 1024;

async fn download_remote_image(
    network: &crate::network::NetworkService,
    url: &str,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>> {
    let response = tokio::select! {
        biased;
        _ = cancellation.cancelled() => return Err(anyhow!("remote image download canceled")),
        response = network.request(
            "GET", url, Default::default(), None, Some(30_000), Some(MAX_REMOTE_IMAGE_BYTES),
        ) => response.map_err(|_| anyhow!("remote image could not be downloaded"))?,
    };
    if !(200..300).contains(&response.status_code) {
        return Err(anyhow!(
            "remote image returned HTTP {}",
            response.status_code
        ));
    }
    Ok(response.body)
}

#[cfg(test)]
#[path = "remote_image_tests.rs"]
mod tests;

/// Shares cancellation and progress with ordinary attachment transfers. Dropping
/// an owner-scoped request must not leave an active transfer behind.
struct RemoteTransfer {
    state: Arc<super::super::AccountState>,
    message_id: String,
    cancellation: CancellationToken,
}

impl Drop for RemoteTransfer {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.state
            .attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.message_id);
        self.state.remove_attachment_progress(&self.message_id);
    }
}

struct TemporaryImage(std::path::PathBuf);

impl Drop for TemporaryImage {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = std::fs::remove_file(&self.0);
        #[cfg(target_arch = "wasm32")]
        {
            let path = self.0.clone();
            crate::task::spawn(async move {
                let _ = crate::fs::remove_file(path).await;
            });
        }
    }
}

fn remote_image_metadata(bytes: &[u8]) -> Result<(&'static str, &'static str, i32, i32)> {
    if bytes.len() > MAX_REMOTE_IMAGE_BYTES {
        return Err(anyhow!("remote image exceeds the size limit"));
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let format = reader
        .format()
        .ok_or_else(|| anyhow!("unsupported remote image format"))?;
    let (mime_type, extension) = match format {
        image::ImageFormat::Gif => ("image/gif", "gif"),
        image::ImageFormat::Png => ("image/png", "png"),
        image::ImageFormat::WebP => ("image/webp", "webp"),
        image::ImageFormat::Jpeg => ("image/jpeg", "jpg"),
        _ => return Err(anyhow!("unsupported remote image format")),
    };
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(MAX_REMOTE_IMAGE_PIXELS * 8);
    reader.limits(limits);
    let decoder = reader.into_decoder()?;
    use image::ImageDecoder as _;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_REMOTE_IMAGE_PIXELS {
        return Err(anyhow!("remote image exceeds the pixel limit"));
    }
    // Validate the actual image, including truncated/corrupt compressed content.
    let _image = image::DynamicImage::from_decoder(decoder)?;
    Ok((
        mime_type,
        extension,
        i32::try_from(width).context("remote image is too wide")?,
        i32::try_from(height).context("remote image is too tall")?,
    ))
}

impl MessageAccess {
    #[allow(clippy::too_many_arguments)]
    pub async fn send_remote_image(
        &self,
        conversation_id: String,
        url: String,
        preview_url: String,
        width: Option<i32>,
        height: Option<i32>,
        mime_type: String,
        silent: bool,
    ) -> Result<String> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.ensure_can_send(&conversation_id).await?;
        let parsed = url::Url::parse(&url)?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(anyhow!("remote image URL must use HTTP or HTTPS"));
        }
        if width.is_some_and(|value| value <= 0) || height.is_some_and(|value| value <= 0) {
            return Err(anyhow!("remote image dimensions must be positive"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(&conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        let text_category = self.text_category(conversation.owner_id.as_deref()).await?;
        let prefix = text_category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid message category: {text_category}"))?;
        let message_id = Uuid::new_v4().to_string();
        let message = Message {
            message_id: message_id.clone(),
            conversation_id,
            user_id: self.account_id.clone(),
            category: format!("{prefix}_IMAGE"),
            media_url: Some(url),
            media_mime_type: Some(mime_type),
            media_size: Some(0),
            media_width: width,
            media_height: height,
            media_status: MediaStatus::Pending,
            thumb_image: Some(preview_url),
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        self.database
            .message_dao
            .insert_pending_outgoing_message(&message)
            .await
            .map_err(crate::CoreError::from)?;
        self.notify_conversation_changed(&message.conversation_id);
        if let Err(error) = self.complete_remote_image_from_url(&message, silent).await {
            warn!(
                "failed to send remote image {}: {error}",
                message.message_id
            );
        }
        Ok(message_id)
    }

    pub(crate) async fn complete_remote_image_from_url(
        &self,
        message: &Message,
        silent: bool,
    ) -> Result<()> {
        let cancellation = CancellationToken::new();
        {
            let mut transfers = self
                .attachment_downloads
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if transfers.contains_key(&message.message_id) {
                return Err(anyhow!("remote image transfer is already active"));
            }
            transfers.insert(message.message_id.clone(), cancellation.clone());
        }
        let _transfer = RemoteTransfer {
            state: self.state.clone(),
            message_id: message.message_id.clone(),
            cancellation: cancellation.clone(),
        };
        self.set_attachment_progress(&message.message_id, 0, 0);
        let result = tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(anyhow!("remote image transfer canceled")),
            result = async {
                self.database.message_dao.update_media_status(
                    &message.message_id, MediaStatus::Pending,
                ).await?;
                self.notify_conversation_changed(&message.conversation_id);
                self.download_and_complete_remote_image(message, silent, &cancellation).await
            } => result,
        };
        if result.is_err() {
            if let Err(error) = self
                .database
                .message_dao
                .cancel_pending_transfer(&message.message_id)
                .await
            {
                warn!("Marking an accepted remote image as retryable failed: {error}");
            }
            self.notify_conversation_changed(&message.conversation_id);
        }
        result
    }

    async fn download_and_complete_remote_image(
        &self,
        message: &Message,
        silent: bool,
        cancellation: &CancellationToken,
    ) -> Result<()> {
        let url = message
            .media_url
            .as_deref()
            .ok_or_else(|| anyhow!("remote image has no URL"))?;
        let bytes = download_remote_image(&self.network, url, cancellation).await?;
        #[cfg(not(target_arch = "wasm32"))]
        let (temporary, metadata) = tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let metadata = remote_image_metadata(&bytes)?;
            let temporary = TemporaryImage(std::env::temp_dir().join(format!(
                "mixin-remote-image-{}.{}",
                Uuid::new_v4(),
                metadata.1
            )));
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary.0)?;
            file.write_all(&bytes)?;
            file.flush()?;
            Ok::<_, anyhow::Error>((temporary, metadata))
        })
        .await??;
        #[cfg(target_arch = "wasm32")]
        let metadata = remote_image_metadata(&bytes)?;
        let (mime_type, _extension, width, height) = metadata;
        if cancellation.is_cancelled() {
            return Err(anyhow!("remote image transfer canceled"));
        }
        let mut message = message.clone();
        message.media_mime_type = Some(mime_type.to_string());
        #[cfg(target_arch = "wasm32")]
        let temporary = TemporaryImage(std::env::temp_dir().join(format!(
            "mixin-remote-image-{}.{}",
            Uuid::new_v4(),
            _extension
        )));
        #[cfg(target_arch = "wasm32")]
        {
            let mut file = crate::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary.0)
                .await?;
            file.write_all(&bytes).await?;
            file.flush().await?;
        }
        self.complete_remote_image_file(&message, &temporary.0, width, height, silent, cancellation)
            .await
    }

    async fn complete_remote_image_file(
        &self,
        message: &Message,
        source: &Path,
        width: i32,
        height: i32,
        silent: bool,
        cancellation: &CancellationToken,
    ) -> Result<()> {
        let mime_type = message.media_mime_type.as_deref().unwrap_or("image/gif");
        let (local_path, media_size) = self
            .app_service
            .attachment
            .import_local(source, message)
            .await?;
        let prefix = message
            .category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid message category: {}", message.category))?;
        let progress_state = self.state.clone();
        let progress_message_id = message.message_id.clone();
        let progress = Arc::new(move |completed, total| {
            progress_state.set_attachment_progress(&progress_message_id, completed, total);
        });
        let upload = self
            .app_service
            .attachment
            .upload(
                &local_path,
                prefix != "PLAIN",
                Some(cancellation),
                Some(progress),
            )
            .await?;
        if cancellation.is_cancelled() {
            return Err(anyhow!("remote image upload canceled"));
        }
        let attachment = AttachmentMessage {
            key: upload.key.clone(),
            digest: upload.digest.clone(),
            attachment_id: upload.attachment_id,
            mime_type: mime_type.to_string(),
            size: media_size,
            name: None,
            width: Some(width),
            height: Some(height),
            thumbnail: message.thumb_image.clone(),
            duration: None,
            waveform: None,
            caption: None,
            created_at: Some(upload.created_at),
            shareable: Some(true),
        };
        let content = Base64::encode_string(serde_json::to_string(&attachment)?.as_bytes());
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(&message.conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {}", message.conversation_id))?;
        let job = Job::create_sending_job(
            &message.message_id,
            &message.conversation_id,
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
                &message.message_id,
                attachment_file_name(&local_path)?,
                &AttachmentMessageUpdate {
                    status: MessageStatus::Sending,
                    content,
                    media_mime_type: mime_type.to_string(),
                    media_size,
                    media_status: MediaStatus::Done,
                    media_width: Some(width),
                    media_height: Some(height),
                    media_digest: upload.digest,
                    media_key: upload.key,
                    media_waveform: None,
                    caption: None,
                    name: None,
                    thumb_image: message.thumb_image.clone(),
                    media_duration: None,
                },
                &job,
            )
            .await?;
        if !completed {
            return Err(anyhow!("remote image send was canceled"));
        }
        // The durable message/job transaction is the acceptance boundary. A wake
        // failure cannot turn a committed upload back into a retryable upload.
        if let Err(error) = self.app_service.job.wake(&job.action) {
            warn!("Waking a committed remote image job failed: {error}");
        }
        self.notify_conversation_changed(&message.conversation_id);
        Ok(())
    }
}
