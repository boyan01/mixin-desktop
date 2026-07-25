use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::{DateTime, Utc};
use log::warn;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sdk::message_category::{MessageCategory as _, MESSAGE_PIN};
use sdk::{
    AttachmentMessage, ContactMessage, ConversationCategory, LiveMessage, MessageStatus,
    PinMessagePayload, StickerMessage,
};

use crate::core::attachment::{attachment_file_name, attachment_path, transcript_attachment_path};
use crate::core::model::job::sanitize_transcript_app_card;
use crate::core::model::AttachmentExtra;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::{AttachmentMessageUpdate, MediaStatus, Message};
use crate::db::mixin::pin_message::PinMessageMinimal;
use crate::db::mixin::transcript_message::TranscriptMessage;
use crate::db::path::account_data_directory;

use super::{
    combine_transcript_category, forward_category, forward_transcript_category, model,
    validate_combine_forward_source, AccountState, AttachmentAccess, MAX_AUDIO_DURATION_MILLIS,
    MAX_AUDIO_FILE_SIZE, MAX_AUDIO_WAVEFORM_SAMPLES,
};

pub struct MessageAccess {
    state: Arc<AccountState>,
}

impl MessageAccess {
    fn message_list_view(
        &self,
        item: crate::db::mixin::message::MessageListItem,
    ) -> Result<model::MessageListView> {
        let mut view: model::MessageListView = item.into();
        self.normalize_local_media_url(&mut view, false)?;
        self.normalize_quote_local_media_url(&mut view, false)?;
        Ok(view)
    }

    fn transcript_message_list_view(
        &self,
        item: crate::db::mixin::transcript_message::TranscriptMessageListItem,
    ) -> Result<model::MessageListView> {
        let mut view: model::MessageListView = item.into();
        self.normalize_local_media_url(&mut view, true)?;
        self.normalize_quote_local_media_url(&mut view, true)?;
        Ok(view)
    }

    fn image_message_view(
        &self,
        item: crate::db::mixin::message::ImageMessageItem,
        conversation_id: &str,
    ) -> Result<model::ImageMessageView> {
        let media_url = if std::path::Path::new(&item.media_url).is_absolute() {
            item.media_url.clone()
        } else {
            attachment_path(
                &account_data_directory(&self.profile.borrow().identity_number)?,
                &Message {
                    message_id: item.message_id.clone(),
                    conversation_id: conversation_id.to_string(),
                    category: "PLAIN_IMAGE".to_string(),
                    media_mime_type: item.media_mime_type.clone(),
                    name: item.media_name.clone(),
                    ..Message::default()
                },
            )?
            .to_string_lossy()
            .into_owned()
        };
        Ok(model::ImageMessageView {
            message_id: item.message_id,
            created_at_micros: item.created_at.and_utc().timestamp_micros(),
            media_url,
            media_name: item.media_name,
            thumb_image: item.thumb_image,
            can_forward: item.can_forward,
            user_id: item.user_id,
            user_full_name: item.user_full_name,
            user_identity_number: item.user_identity_number,
            avatar_url: item.avatar_url,
        })
    }

    fn normalize_local_media_url(
        &self,
        view: &mut model::MessageListView,
        is_transcript: bool,
    ) -> Result<()> {
        let Some(media_url) = view.media_url.as_deref() else {
            return Ok(());
        };
        if media_url.trim().is_empty()
            || std::path::Path::new(media_url).is_absolute()
            || !view.category.is_attachment()
        {
            return Ok(());
        }
        let message = Message {
            message_id: view.message_id.clone(),
            conversation_id: view.conversation_id.clone(),
            category: view.category.clone(),
            media_mime_type: view.media_mime_type.clone(),
            name: view.media_name.clone(),
            ..Message::default()
        };
        let account_data_dir = account_data_directory(&self.profile.borrow().identity_number)?;
        let path = if is_transcript {
            transcript_attachment_path(&account_data_dir, &message)?
        } else {
            attachment_path(&account_data_dir, &message)?
        };
        view.media_url = Some(path.to_string_lossy().into_owned());
        Ok(())
    }

    fn normalize_quote_local_media_url(
        &self,
        view: &mut model::MessageListView,
        is_transcript: bool,
    ) -> Result<()> {
        let Some(content) = view.quote_content.as_deref() else {
            return Ok(());
        };
        if let Some(content) = normalized_quote_content(content, is_transcript, || {
            account_data_directory(&self.profile.borrow().identity_number)
        })? {
            view.quote_content = Some(content);
        }
        Ok(())
    }

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
            .await?;
        self.notify_conversation_changed(&message.conversation_id);
        if let Err(error) = self.complete_remote_image_from_url(&message, silent).await {
            warn!(
                "failed to send remote image {}: {error}",
                message.message_id
            );
            self.database
                .message_dao
                .update_media_status(&message.message_id, MediaStatus::Canceled)
                .await?;
            self.notify_conversation_changed(&message.conversation_id);
        }
        Ok(message_id)
    }

    pub(crate) async fn complete_remote_image_from_url(
        &self,
        message: &Message,
        silent: bool,
    ) -> Result<()> {
        let url = message
            .media_url
            .as_deref()
            .ok_or_else(|| anyhow!("remote image has no URL"))?;
        let bytes = self.app_service.attachment.download_public(url).await?;
        let image = image::ImageReader::new(Cursor::new(bytes.as_slice()))
            .with_guessed_format()?
            .decode()?;
        let width = i32::try_from(image.width()).context("remote image is too wide")?;
        let height = i32::try_from(image.height()).context("remote image is too tall")?;
        let extension = match message.media_mime_type.as_deref() {
            Some("image/gif") => "gif",
            Some("image/png") => "png",
            Some("image/webp") => "webp",
            _ => "jpg",
        };
        let temporary = std::env::temp_dir().join(format!(
            "mixin-remote-image-{}.{}",
            message.message_id, extension
        ));
        tokio::fs::write(&temporary, bytes).await?;
        let result = self
            .complete_remote_image_file(message, &temporary, width, height, silent)
            .await;
        let _ = tokio::fs::remove_file(&temporary).await;
        result
    }

    async fn complete_remote_image_file(
        &self,
        message: &Message,
        source: &Path,
        width: i32,
        height: i32,
        silent: bool,
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
        let upload = self
            .app_service
            .attachment
            .upload(&local_path, prefix != "PLAIN", None, None)
            .await?;
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
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(&message.conversation_id);
        Ok(())
    }

    pub(crate) fn new(state: Arc<AccountState>) -> Self {
        Self { state }
    }
}

impl Deref for MessageAccess {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl MessageAccess {
    pub async fn messages(
        &self,
        conversation_id: String,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<String>,
        limit: i64,
    ) -> Result<Vec<model::MessageListView>> {
        let before_message_id = before_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        if before_created_at_micros.is_some() != before_message_id.is_some() {
            return Err(anyhow!(
                "message cursor requires both timestamp and message id"
            ));
        }
        let before_created_at_millis =
            before_created_at_micros.map(|value| value.div_euclid(1_000));
        Ok(self
            .database
            .message_dao
            .list_items(
                conversation_id,
                before_created_at_millis,
                before_message_id,
                limit,
            )
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()?)
    }

    pub async fn message_items_by_ids(
        &self,
        message_ids: Vec<String>,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .list_items_by_ids(&message_ids)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()?)
    }

    pub async fn message_order_info(
        &self,
        message_id: String,
    ) -> Result<Option<model::MessageOrderInfoView>> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .message_order_info(&message_id)
            .await?
            .map(|info| model::MessageOrderInfoView {
                message_id: info.message_id,
                row_id: info.row_id,
                created_at_micros: info.created_at.and_utc().timestamp_micros(),
            }))
    }

    pub async fn message_ids_before(
        &self,
        conversation_id: String,
        anchor_row_id: i64,
        anchor_created_at_micros: i64,
        limit: i64,
    ) -> Result<Vec<String>> {
        self.ensure_active()?;
        let created_at = DateTime::from_timestamp_micros(anchor_created_at_micros)
            .ok_or_else(|| anyhow!("invalid message timestamp"))?
            .naive_utc();
        self.database
            .message_dao
            .message_ids_before(
                &conversation_id,
                &crate::db::mixin::message::MessageOrderInfo {
                    message_id: String::new(),
                    row_id: anchor_row_id,
                    created_at,
                },
                limit,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn message_ids_after(
        &self,
        conversation_id: String,
        anchor_row_id: i64,
        anchor_created_at_micros: i64,
        limit: i64,
    ) -> Result<Vec<String>> {
        self.ensure_active()?;
        let created_at = DateTime::from_timestamp_micros(anchor_created_at_micros)
            .ok_or_else(|| anyhow!("invalid message timestamp"))?
            .naive_utc();
        self.database
            .message_dao
            .message_ids_after(
                &conversation_id,
                &crate::db::mixin::message::MessageOrderInfo {
                    message_id: String::new(),
                    row_id: anchor_row_id,
                    created_at,
                },
                limit,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn search_messages(
        &self,
        conversation_id: String,
        query: String,
        sender_id: Option<String>,
        categories: Vec<String>,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>> {
        let sender_id = sender_id.as_deref();
        self.search_message_items(
            Some(conversation_id.as_str()),
            query.as_str(),
            sender_id,
            categories.as_slice(),
            anchor_message_id.as_deref(),
            limit,
        )
        .await
    }

    pub async fn search_global_messages(
        &self,
        query: String,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>> {
        self.search_message_items(
            None,
            query.as_str(),
            None,
            &[],
            anchor_message_id.as_deref(),
            limit,
        )
        .await
    }

    async fn search_message_items(
        &self,
        conversation_id: Option<&str>,
        query: &str,
        sender_id: Option<&str>,
        categories: &[String],
        anchor_message_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        let matches = self
            .database
            .message_fts_dao
            .search(
                query,
                conversation_id,
                sender_id,
                categories,
                anchor_message_id,
                limit,
            )
            .await?;
        let message_ids = matches
            .iter()
            .map(|item| item.message_id.clone())
            .collect::<Vec<_>>();
        let mut items_by_id = self
            .database
            .message_dao
            .list_items_by_ids(&message_ids)
            .await?
            .into_iter()
            .map(|item| (item.message_id.clone(), item))
            .collect::<HashMap<_, _>>();
        let mut result = Vec::with_capacity(matches.len());
        for matched in matches {
            if let Some(item) = items_by_id.remove(&matched.message_id) {
                result.push(self.message_list_view(item)?);
            }
        }
        Ok(result)
    }

    pub async fn shared_messages(
        &self,
        conversation_id: String,
        kind: String,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<model::MessageListView>> {
        let conversation_id = conversation_id.as_str();
        let kind = kind.as_str();
        self.ensure_active()?;
        let page_size = limit.clamp(1, 200);
        let target = offset.saturating_add(page_size);
        let mut result = Vec::new();
        let mut cursor: Option<(i64, String)> = None;
        while result.len() < target {
            let page = self
                .messages(
                    conversation_id.to_string(),
                    cursor.as_ref().map(|value| value.0),
                    cursor.as_ref().map(|value| value.1.clone()),
                    200,
                )
                .await?;
            if page.is_empty() {
                break;
            }
            for item in &page {
                let category = item.category.as_str();
                let matches = match kind {
                    "media" => matches!(
                        category,
                        "PLAIN_IMAGE" | "SIGNAL_IMAGE" | "PLAIN_VIDEO" | "SIGNAL_VIDEO"
                    ),
                    "post" => matches!(category, "PLAIN_POST" | "SIGNAL_POST"),
                    "file" => matches!(category, "PLAIN_DATA" | "SIGNAL_DATA"),
                    _ => false,
                };
                if matches {
                    result.push(item.clone());
                    if result.len() >= target {
                        break;
                    }
                }
            }
            let Some(last) = page.last() else { break };
            cursor = Some((last.created_at_micros, last.message_id.clone()));
            if page.len() < 200 {
                break;
            }
        }
        Ok(result.into_iter().skip(offset).take(page_size).collect())
    }

    pub async fn messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>> {
        let conversation_id = conversation_id.as_str();
        let target_message_id = target_message_id.as_str();
        Ok(self
            .database
            .message_dao
            .list_items_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()?)
    }

    pub async fn image_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::ImageMessageView>> {
        let conversation_id = conversation_id.as_str();
        let target_message_id = target_message_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .list_image_items_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.image_message_view(item, conversation_id))
            .collect::<Result<Vec<_>>>()?)
    }

    pub async fn pinned_messages(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::MessageListView>> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .list_pinned_items(conversation_id)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()?)
    }

    pub async fn pinned_message_ids(&self, conversation_id: String) -> Result<Vec<String>> {
        self.ensure_active()?;
        Ok(self
            .database
            .pin_message_dao
            .message_ids(&conversation_id)
            .await?)
    }

    pub async fn pin_message_preview(
        &self,
        conversation_id: String,
    ) -> Result<Option<model::PinMessagePreviewItem>> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .pin_message_dao
            .latest_preview(conversation_id)
            .await?
            .map(|item| model::PinMessagePreviewItem {
                message_id: item.message_id,
                content: item.content.unwrap_or_default(),
                sender_name: item.sender_name.unwrap_or_default(),
            }))
    }

    pub async fn transcript_messages(
        &self,
        transcript_id: String,
    ) -> Result<Vec<model::MessageListView>> {
        let transcript_id = transcript_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .transcript_message_dao
            .list_items(transcript_id)
            .await?
            .into_iter()
            .map(|item| self.transcript_message_list_view(item))
            .collect::<Result<Vec<_>>>()?)
    }

    pub async fn send_text(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String> {
        self.send_text_message(conversation_id, content, quote_message_id, false, silent)
            .await
    }

    pub async fn conversation_is_encrypted(&self, conversation_id: String) -> Result<bool> {
        self.ensure_active()?;
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(&conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        Ok(self.text_category(conversation.owner_id.as_deref()).await?
            != sdk::message_category::PLAIN_TEXT)
    }

    pub async fn send_post(&self, conversation_id: String, content: String) -> Result<String> {
        self.send_text_message(conversation_id, content, None, true, false)
            .await
    }

    pub async fn send_app_card(&self, conversation_id: String, content: String) -> Result<String> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        let mut card = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&content)
            .context("invalid app card")?;
        if !card.get("title").is_some_and(serde_json::Value::is_string)
            || !card
                .get("description")
                .is_some_and(serde_json::Value::is_string)
        {
            return Err(anyhow!("app card title and description are required"));
        }
        if card.get("action").and_then(serde_json::Value::as_str) == Some("") {
            card.insert("action".into(), serde_json::Value::Null);
        }
        if card
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|actions| {
                actions.iter().any(|action| {
                    action
                        .get("action")
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|action| !is_shareable_app_card_action(action))
                })
            })
        {
            card.insert("actions".into(), serde_json::Value::Null);
        }
        let content = serde_json::to_string(&card)?;
        let message_id = Uuid::new_v4().to_string();
        let message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category: sdk::message_category::APP_CARD.into(),
            content: Some(content),
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        let job = Job::create_sending_job(
            &message_id,
            conversation_id,
            None,
            None,
            false,
            false,
            conversation.expire_in,
        );
        self.database
            .message_dao
            .insert_outgoing_message(&message, &job)
            .await?;
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(conversation_id);
        Ok(message_id)
    }

    async fn send_text_message(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
        post: bool,
        silent: bool,
    ) -> Result<String> {
        let quote_message_id = quote_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        let content = content.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if content.is_empty() {
            return Err(anyhow!("message content is empty"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        let mut category = self.text_category(conversation.owner_id.as_deref()).await?;
        if post {
            category = category.replace("_TEXT", "_POST");
        }
        let message_id = Uuid::new_v4().to_string();
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
        let message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category,
            content: Some(content.to_string()),
            quote_message_id: quote_message_id.map(str::to_string),
            quote_content,
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        let job = Job::create_sending_job(
            &message_id,
            conversation_id,
            None,
            None,
            false,
            silent,
            conversation.expire_in,
        );
        self.database
            .message_dao
            .insert_outgoing_message(&message, &job)
            .await?;
        if let Err(error) = self
            .database
            .message_fts_dao
            .upsert(&message_id, conversation_id, content)
            .await
        {
            warn!("failed to index outgoing message {message_id}: {error}");
        }
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(conversation_id);
        Ok(message_id)
    }

    pub async fn send_contact(
        &self,
        conversation_id: String,
        shared_user_id: String,
        quote_message_id: Option<String>,
        silent: bool,
    ) -> Result<String> {
        let conversation_id = conversation_id.as_str();
        let shared_user_id = shared_user_id.as_str();
        let quote_message_id = quote_message_id.as_deref();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        let category = self
            .text_category(conversation.owner_id.as_deref())
            .await?
            .replace("_TEXT", "_CONTACT");
        let content = Base64::encode_string(
            serde_json::to_string(&ContactMessage {
                user_id: shared_user_id.to_string(),
            })?
            .as_bytes(),
        );
        let message_id = Uuid::new_v4().to_string();
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
        let message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category,
            content: Some(content),
            shared_user_id: Some(shared_user_id.to_string()),
            quote_message_id: quote_message_id.map(str::to_string),
            quote_content,
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        let job = Job::create_sending_job(
            &message_id,
            conversation_id,
            None,
            None,
            false,
            silent,
            conversation.expire_in,
        );
        self.database
            .message_dao
            .insert_outgoing_message(&message, &job)
            .await?;
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(conversation_id);
        Ok(message_id)
    }

    pub async fn send_sticker(
        &self,
        conversation_id: String,
        sticker_id: String,
    ) -> Result<String> {
        let conversation_id = conversation_id.as_str();
        let sticker_id = sticker_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if sticker_id.trim().is_empty() {
            return Err(anyhow!("sticker id is required"));
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
        let sticker = self
            .database
            .sticker_dao
            .find_sticker_by_id(sticker_id)
            .await?
            .ok_or_else(|| anyhow!("sticker not found: {sticker_id}"))?;
        let album_id = self
            .database
            .sticker_dao
            .find_system_album_for_sticker(sticker_id)
            .await?
            .map(|album| album.album_id)
            .or(sticker.album_id.clone());
        let sticker_message = StickerMessage {
            sticker_id: sticker_id.to_string(),
            album_id: album_id.clone(),
            name: Some(sticker.name.clone()),
        };
        let content = Base64::encode_string(serde_json::to_string(&sticker_message)?.as_bytes());
        let message_id = Uuid::new_v4().to_string();
        let message = Message {
            message_id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category: format!("{prefix}_STICKER"),
            content: Some(content),
            album_id,
            sticker_id: Some(sticker_id.to_string()),
            name: Some(sticker.name),
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        let job = Job::create_sending_job(
            &message_id,
            conversation_id,
            None,
            None,
            false,
            false,
            conversation.expire_in,
        );
        self.database
            .message_dao
            .insert_outgoing_message(&message, &job)
            .await?;
        self.database
            .sticker_dao
            .update_last_used(sticker_id)
            .await?;
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(conversation_id);
        Ok(message_id)
    }

    pub async fn send_audio(
        &self,
        conversation_id: String,
        path: String,
        duration_millis: i64,
        waveform: Vec<u8>,
        quote_message_id: Option<String>,
    ) -> Result<String> {
        let quote_message_id = quote_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        let path = path.as_str();
        let waveform = waveform.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
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
        let source = tokio::fs::canonicalize(path)
            .await
            .with_context(|| format!("resolve recorded audio {path}"))?;
        let metadata = tokio::fs::metadata(&source).await?;
        if !metadata.is_file() || metadata.len() == 0 {
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
            .await?;
        self.notify_conversation_changed(conversation_id);

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
        let quote_message_id = quote_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        let path = path.as_str();
        let kind = kind.trim().to_ascii_uppercase();
        let mime_type = mime_type.trim();
        let name = name.filter(|value| !value.trim().is_empty());
        let caption = caption.filter(|value| !value.trim().is_empty());
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;

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

        let source = tokio::fs::canonicalize(path)
            .await
            .with_context(|| format!("resolve attachment {path}"))?;
        let metadata = tokio::fs::metadata(&source).await?;
        if !metadata.is_file() || metadata.len() == 0 {
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
            .await?;
        self.notify_conversation_changed(conversation_id);

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

    pub async fn forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<Vec<String>> {
        let target_conversation_id = target_conversation_id.as_str();
        let source_message_ids = source_message_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if source_message_ids.is_empty() {
            return Err(anyhow!("forward message ids are empty"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(target_conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {target_conversation_id}"))?;
        let target_text_category = self.text_category(conversation.owner_id.as_deref()).await?;
        let target_prefix = target_text_category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid target message category: {target_text_category}"))?;

        let sources = self.messages_by_ids(source_message_ids).await?;
        for (source, source_message_id) in sources.iter().zip(source_message_ids) {
            if !matches!(
                source.status,
                MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
            ) {
                return Err(anyhow!(
                    "message is not ready to forward: {source_message_id}"
                ));
            }
            forward_category(&source.category, target_prefix)?;
            if source.category == sdk::message_category::APP_CARD {
                let shareable = source
                    .content
                    .as_deref()
                    .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
                    .and_then(|value| value.get("shareable").and_then(serde_json::Value::as_bool))
                    .unwrap_or(false);
                if !shareable {
                    return Err(anyhow!("app card is not shareable: {source_message_id}"));
                }
            } else if source.category.is_attachment() {
                if !matches!(source.media_status, MediaStatus::Done | MediaStatus::Read) {
                    return Err(anyhow!(
                        "attachment is not available locally: {source_message_id}"
                    ));
                }
                let extra = source
                    .content
                    .as_deref()
                    .and_then(|content| serde_json::from_str::<AttachmentExtra>(content).ok())
                    .ok_or_else(|| {
                        anyhow!("attachment metadata is invalid: {source_message_id}")
                    })?;
                if extra.shareable == Some(false) {
                    return Err(anyhow!("attachment is not shareable: {source_message_id}"));
                }
                if source.media_url.as_deref().is_none_or(str::is_empty) {
                    return Err(anyhow!("attachment has no local file: {source_message_id}"));
                }
                if let Some(waveform) = source.media_waveform.as_deref() {
                    Base64::decode_vec(waveform).with_context(|| {
                        format!("attachment waveform is invalid: {source_message_id}")
                    })?;
                }
            } else if source.category.is_live() {
                let live = source
                    .content
                    .as_deref()
                    .and_then(|content| serde_json::from_str::<LiveMessage>(content).ok())
                    .ok_or_else(|| anyhow!("live message is invalid: {source_message_id}"))?;
                if !live.shareable {
                    return Err(anyhow!(
                        "live message is not shareable: {source_message_id}"
                    ));
                }
            }
        }

        let mut forwarded_ids = Vec::with_capacity(sources.len());
        for source in sources {
            let message_id = Uuid::new_v4().to_string();
            let category = forward_category(&source.category, target_prefix)?;
            let mut content = source.content.clone().unwrap_or_default();
            let mut media_size = None;
            let mut media_status = crate::db::mixin::message::MediaStatus::Done;
            let mut media_url = None;
            let mut media_mime_type = None;
            let mut media_duration = String::new();
            let mut media_width = None;
            let mut media_height = None;
            let mut thumb_image = None;
            let mut thumb_url = None;
            let mut media_key = None;
            let mut media_digest = None;
            let mut media_waveform = None;
            let mut caption = None;
            let mut copied_attachment_path = None;
            let mut forwarded_transcripts = None;

            if source.category.is_attachment() {
                let extra = serde_json::from_str::<AttachmentExtra>(
                    source.content.as_deref().unwrap_or_default(),
                )?;
                let stored_path = source.media_url.as_deref().unwrap_or_default();
                let source_path = if Path::new(stored_path).is_absolute() {
                    Path::new(stored_path).to_path_buf()
                } else {
                    attachment_path(
                        &account_data_directory(&self.profile.borrow().identity_number)?,
                        &source,
                    )?
                };
                let target_file_message = Message {
                    message_id: message_id.clone(),
                    conversation_id: target_conversation_id.to_string(),
                    category: category.clone(),
                    media_mime_type: source.media_mime_type.clone(),
                    name: source.name.clone(),
                    ..Message::default()
                };
                let (target_path, actual_size) = self
                    .app_service
                    .attachment
                    .copy_for_forward(&source_path, &target_file_message)
                    .await?;
                copied_attachment_path = Some(target_path.clone());

                let same_prefix = source.category.starts_with(&format!("{target_prefix}_"));
                let reusable_age = extra.created_at.and_then(|created_at| {
                    let age = Utc::now().signed_duration_since(created_at);
                    (age >= chrono::Duration::zero() && age <= chrono::Duration::hours(24))
                        .then_some(age)
                });
                let reusable_material = target_prefix == "PLAIN"
                    || (source.media_key.is_some() && source.media_digest.is_some());
                let (attachment_id, attachment_created_at, key, digest) =
                    if same_prefix && reusable_age.is_some() && reusable_material {
                        (
                            extra.attachment_id,
                            extra.created_at,
                            source.media_key.clone(),
                            source.media_digest.clone(),
                        )
                    } else {
                        let upload = match self
                            .app_service
                            .attachment
                            .upload(&target_path, target_prefix != "PLAIN", None, None)
                            .await
                        {
                            Ok(upload) => upload,
                            Err(error) => {
                                let _ = tokio::fs::remove_file(&target_path).await;
                                return Err(error);
                            }
                        };
                        (
                            upload.attachment_id,
                            Some(upload.created_at),
                            upload.key,
                            upload.digest,
                        )
                    };
                let attachment = AttachmentMessage {
                    key: key.clone(),
                    digest: digest.clone(),
                    attachment_id,
                    mime_type: source
                        .media_mime_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                    size: actual_size,
                    name: source.name.clone(),
                    width: source.media_width,
                    height: source.media_height,
                    thumbnail: source.thumb_image.clone(),
                    duration: source.media_duration.parse().ok(),
                    waveform: source
                        .media_waveform
                        .as_deref()
                        .map(Base64::decode_vec)
                        .transpose()?,
                    caption: source.caption.clone(),
                    created_at: attachment_created_at,
                    shareable: Some(true),
                };
                content = Base64::encode_string(serde_json::to_string(&attachment)?.as_bytes());
                media_url = Some(attachment_file_name(&target_path)?.to_string());
                media_mime_type = source.media_mime_type.clone();
                media_size = Some(actual_size);
                media_duration = source.media_duration.clone();
                media_width = source.media_width;
                media_height = source.media_height;
                thumb_image = source.thumb_image.clone();
                media_key = key;
                media_digest = digest;
                media_waveform = source.media_waveform.clone();
                caption = source.caption.clone();
            } else if source.category.is_live() {
                let live = LiveMessage {
                    width: source
                        .media_width
                        .ok_or_else(|| anyhow!("forward live message has no width"))?,
                    height: source
                        .media_height
                        .ok_or_else(|| anyhow!("forward live message has no height"))?,
                    thumb_url: source.thumb_url.clone().unwrap_or_default(),
                    url: source
                        .media_url
                        .clone()
                        .filter(|url| !url.is_empty())
                        .ok_or_else(|| anyhow!("forward live message has no URL"))?,
                    shareable: true,
                };
                let live_content = serde_json::to_string(&live)?;
                content = live_content;
                media_url = Some(live.url);
                thumb_url = Some(live.thumb_url);
                media_width = Some(live.width);
                media_height = Some(live.height);
            } else if source.category.is_sticker() {
                let sticker = StickerMessage {
                    sticker_id: source.sticker_id.clone().ok_or_else(|| {
                        anyhow!("forward sticker has no sticker id: {}", source.message_id)
                    })?,
                    album_id: None,
                    name: None,
                };
                content = Base64::encode_string(serde_json::to_string(&sticker)?.as_bytes());
            } else if source.category.is_contact() {
                let contact = ContactMessage {
                    user_id: source.shared_user_id.clone().ok_or_else(|| {
                        anyhow!(
                            "forward contact has no shared user id: {}",
                            source.message_id
                        )
                    })?,
                };
                content = Base64::encode_string(serde_json::to_string(&contact)?.as_bytes());
            } else if source.category.is_transcript() {
                let transcripts = self
                    .database
                    .transcript_message_dao
                    .find_by_transcript_id(&source.message_id)
                    .await?;
                if transcripts.is_empty() {
                    return Err(anyhow!("transcript is empty: {}", source.message_id));
                }
                if transcripts
                    .iter()
                    .any(|transcript| transcript.category.is_attachment())
                {
                    return Err(anyhow!(
                        "transcript attachments cannot be forwarded yet: {}",
                        source.message_id
                    ));
                }
                let mut transcript_entries = Vec::with_capacity(transcripts.len());
                let mut summaries = Vec::with_capacity(transcripts.len());
                for transcript in transcripts {
                    let transcript_category =
                        forward_transcript_category(&transcript.category, target_prefix)?;
                    summaries.push(serde_json::json!({
                        "name": transcript.user_full_name.as_deref().unwrap_or_default(),
                        "category": transcript_category,
                        "content": transcript.content,
                    }));
                    transcript_entries.push(TranscriptMessage {
                        transcript_id: message_id.clone(),
                        category: transcript_category,
                        ..transcript
                    });
                }
                content = serde_json::to_string(&summaries)?;
                media_size = Some(0);
                media_status = crate::db::mixin::message::MediaStatus::Done;
                forwarded_transcripts = Some(transcript_entries);
            } else if source.content.is_none() {
                return Err(anyhow!(
                    "forward message has no content: {}",
                    source.message_id
                ));
            }

            let message = Message {
                message_id: message_id.clone(),
                conversation_id: target_conversation_id.to_string(),
                user_id: self.account_id.clone(),
                category,
                content: Some(content.clone()),
                media_url,
                media_mime_type,
                media_size,
                media_duration,
                media_width,
                media_height,
                thumb_image,
                media_key,
                media_digest,
                media_status,
                name: source.name,
                album_id: source.album_id,
                sticker_id: source.sticker_id,
                shared_user_id: source.shared_user_id,
                media_waveform,
                thumb_url,
                caption,
                status: MessageStatus::Sending,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            };
            let job = Job::create_sending_job(
                &message_id,
                target_conversation_id,
                None,
                None,
                false,
                false,
                conversation.expire_in,
            );
            let inserted = match forwarded_transcripts.as_deref() {
                Some(transcripts) => {
                    self.database
                        .message_dao
                        .insert_outgoing_message_with_transcripts(&message, &job, transcripts)
                        .await
                }
                None => {
                    self.database
                        .message_dao
                        .insert_outgoing_message(&message, &job)
                        .await
                }
            };
            if let Err(error) = inserted {
                if let Some(path) = copied_attachment_path {
                    let _ = tokio::fs::remove_file(path).await;
                }
                return Err(error.into());
            }
            if source.category.is_text()
                || source.category.is_post()
                || source.category.is_transcript()
            {
                if let Err(error) = self
                    .database
                    .message_fts_dao
                    .upsert(&message_id, target_conversation_id, &content)
                    .await
                {
                    warn!("failed to index forwarded message {message_id}: {error}");
                }
            }
            forwarded_ids.push(message_id);
        }
        self.app_service.job.wake(sdk::SENDING_MESSAGE)?;
        self.notify_conversation_changed(target_conversation_id);
        Ok(forwarded_ids)
    }

    pub async fn combine_forward_messages(
        &self,
        target_conversation_id: String,
        source_message_ids: Vec<String>,
    ) -> Result<String> {
        let target_conversation_id = target_conversation_id.as_str();
        let source_message_ids = source_message_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if !(2..100).contains(&source_message_ids.len()) {
            return Err(anyhow!("combine forward requires 2 to 99 messages"));
        }
        let unique_ids = source_message_ids.iter().collect::<HashSet<_>>();
        if unique_ids.len() != source_message_ids.len() {
            return Err(anyhow!("combine forward message ids contain duplicates"));
        }
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(target_conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {target_conversation_id}"))?;
        let target_text_category = self.text_category(conversation.owner_id.as_deref()).await?;
        let target_prefix = target_text_category
            .split_once('_')
            .map(|(prefix, _)| prefix)
            .ok_or_else(|| anyhow!("invalid target message category: {target_text_category}"))?;

        let mut sources = self.messages_by_ids(source_message_ids).await?;
        for source in &sources {
            validate_combine_forward_source(source)?;
        }
        sources.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.message_id.cmp(&right.message_id))
        });
        let user_ids = sources
            .iter()
            .flat_map(|source| {
                std::iter::once(source.user_id.clone()).chain(source.shared_user_id.iter().cloned())
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let user_names = self
            .database
            .user_dao
            .find_users(&user_ids)
            .await?
            .into_iter()
            .map(|user| (user.user_id, user.full_name))
            .collect::<HashMap<_, _>>();

        let transcript_id = Uuid::new_v4().to_string();
        let mut transcripts = Vec::with_capacity(sources.len());
        let mut summaries = Vec::with_capacity(sources.len());
        let mut fts_parts = Vec::new();
        let mut has_attachments = false;
        for source in sources {
            let sender_name = user_names.get(&source.user_id).cloned().unwrap_or_default();
            let category = combine_transcript_category(&source.category, target_prefix)?;
            let is_attachment = source.category.is_attachment();
            has_attachments |= is_attachment;
            let media_created_at = source
                .content
                .as_deref()
                .and_then(|content| serde_json::from_str::<AttachmentExtra>(content).ok())
                .and_then(|extra| extra.created_at);
            let mut transcript = TranscriptMessage {
                transcript_id: transcript_id.clone(),
                message_id: source.message_id,
                user_id: Some(source.user_id),
                user_full_name: Some(sender_name.clone()),
                category: category.clone(),
                created_at: source.created_at.and_utc(),
                content: source.content,
                media_url: source.media_url,
                media_name: source.name,
                media_size: source.media_size,
                media_width: source.media_width,
                media_height: source.media_height,
                media_mime_type: source.media_mime_type,
                media_duration: Some(source.media_duration),
                media_status: Some(if is_attachment {
                    MediaStatus::Canceled
                } else {
                    source.media_status
                }),
                media_waveform: source.media_waveform,
                thumb_image: source.thumb_image,
                thumb_url: source.thumb_url,
                media_key: source.media_key.map(|key| Base64::encode_string(&key)),
                media_digest: source
                    .media_digest
                    .map(|digest| Base64::encode_string(&digest)),
                media_created_at,
                sticker_id: source.sticker_id,
                shared_user_id: source.shared_user_id,
                mentions: None,
                quote_id: source.quote_message_id,
                quote_content: source.quote_content,
                caption: source.caption,
            };
            sanitize_transcript_app_card(&mut transcript);
            if category.is_text() || category.is_post() {
                if let Some(content) = transcript.content.as_deref() {
                    fts_parts.push(content.to_string());
                }
            } else if category.is_data() {
                if let Some(name) = transcript.media_name.as_deref() {
                    fts_parts.push(name.to_string());
                }
            } else if category.is_contact() {
                if let Some(user_id) = transcript.shared_user_id.as_deref() {
                    if let Some(full_name) = user_names.get(user_id) {
                        fts_parts.push(full_name.clone());
                    }
                }
            }
            summaries.push(serde_json::json!({
                "name": sender_name,
                "category": category,
                "content": transcript.content,
            }));
            transcripts.push(transcript);
        }

        let content = serde_json::to_string(&summaries)?;
        let message = Message {
            message_id: transcript_id.clone(),
            conversation_id: target_conversation_id.to_string(),
            user_id: self.account_id.clone(),
            category: format!("{target_prefix}_TRANSCRIPT"),
            content: Some(content.clone()),
            media_size: Some(0),
            media_status: if has_attachments {
                MediaStatus::Canceled
            } else {
                MediaStatus::Done
            },
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        };
        let job = Job::create_sending_job(
            &transcript_id,
            target_conversation_id,
            None,
            None,
            false,
            false,
            conversation.expire_in,
        );
        self.database
            .message_dao
            .insert_outgoing_message_with_transcripts(&message, &job, &transcripts)
            .await?;
        let fts_content = fts_parts.join(" ");
        if !fts_content.is_empty() {
            if let Err(error) = self
                .database
                .message_fts_dao
                .upsert(&transcript_id, target_conversation_id, &fts_content)
                .await
            {
                warn!("failed to index combined transcript {transcript_id}: {error}");
            }
        }
        if has_attachments {
            AttachmentAccess::new(self.state.clone())
                .retry_transcript_attachment(transcript_id.clone())
                .await?;
        } else {
            self.app_service.job.wake(&job.action)?;
        }
        self.notify_conversation_changed(target_conversation_id);
        Ok(transcript_id)
    }

    pub async fn delete_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let message_ids = message_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let messages = self.messages_by_ids(message_ids).await?;
        for message in &messages {
            if message.conversation_id != conversation_id {
                return Err(anyhow!("message does not belong to conversation"));
            }
        }
        self.database
            .message_dao
            .delete_messages_batch(conversation_id, message_ids)
            .await?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn recall_messages(
        &self,
        conversation_id: String,
        message_ids: Vec<String>,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let message_ids = message_ids.as_slice();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let recall_deadline = Utc::now().naive_utc() - chrono::Duration::minutes(60);
        let messages = self.messages_by_ids(message_ids).await?;
        for (message, message_id) in messages.iter().zip(message_ids) {
            let sent = matches!(
                message.status,
                MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
            );
            if message.conversation_id != conversation_id
                || message.user_id != self.account_id
                || !sent
                || !message.category.can_recall()
                || message.created_at < recall_deadline
            {
                return Err(anyhow!("message can not be recalled: {message_id}"));
            }
        }
        let jobs = message_ids
            .iter()
            .map(|message_id| Job::create_send_recall_job(conversation_id, message_id))
            .collect::<Vec<_>>();
        self.database
            .message_dao
            .recall_messages_with_jobs(conversation_id, message_ids, &jobs)
            .await?;
        self.app_service.job.wake(sdk::RECALL_MESSAGE)?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn set_message_pinned(
        &self,
        conversation_id: String,
        message_id: String,
        pinned: bool,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?;
        let sent = matches!(
            message.status,
            MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
        );
        if message.conversation_id != conversation_id || !message.category.can_reply() || !sent {
            return Err(anyhow!("message can not be pinned: {message_id}"));
        }
        let participant = self
            .database
            .participant_dao
            .find_participant_by_id(conversation_id, &self.account_id)
            .await?;
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        if conversation.category == Some(ConversationCategory::Group)
            && participant.and_then(|item| item.role).is_none()
        {
            return Err(anyhow!("current user can not pin messages"));
        }
        let payload = if pinned {
            PinMessagePayload::Pin(vec![message_id.to_string()])
        } else {
            PinMessagePayload::Unpin(vec![message_id.to_string()])
        };
        let encoded = serde_json::to_string(&payload)?;
        let job = Job::create_send_pin_job(conversation_id, &encoded);
        let now = Utc::now();
        let pin_event = pinned
            .then(|| {
                Ok::<_, serde_json::Error>(Message {
                    message_id: Uuid::new_v4().to_string(),
                    conversation_id: conversation_id.to_string(),
                    user_id: self.account_id.clone(),
                    category: MESSAGE_PIN.to_string(),
                    content: Some(serde_json::to_string(&PinMessageMinimal {
                        category: message.category.clone(),
                        message_id: message.message_id.clone(),
                        content: if message.category.is_text() {
                            message.content.clone()
                        } else {
                            None
                        },
                    })?),
                    status: MessageStatus::Read,
                    created_at: now.naive_utc(),
                    quote_message_id: Some(message.message_id.clone()),
                    ..Message::default()
                })
            })
            .transpose()?;
        self.database
            .message_dao
            .set_message_pinned_with_job(
                conversation_id,
                message_id,
                pinned,
                now,
                pin_event.as_ref(),
                &job,
            )
            .await?;
        self.app_service.job.wake(sdk::PIN_MESSAGE)?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn mark_mention_read(
        &self,
        conversation_id: String,
        message_id: String,
    ) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let message_id = message_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let ids = [message_id.to_string()];
        if self
            .database
            .message_mention_dao
            .mark_mention_read(&ids)
            .await?
            == 0
        {
            return Ok(());
        }
        let job = Job::create_mention_read_ack_job(conversation_id, message_id);
        self.database.job_dao.insert_job(&job).await?;
        self.app_service.job.wake(sdk::CREATE_MESSAGE)?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn unread_mention_message_ids(&self, conversation_id: String) -> Result<Vec<String>> {
        let conversation_id = conversation_id.as_str();
        self.ensure_active()?;
        Ok(self
            .database
            .message_mention_dao
            .unread_message_ids(conversation_id)
            .await?)
    }

    pub async fn mark_conversation_read(&self, conversation_id: String) -> Result<()> {
        let conversation_id = conversation_id.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let (messages_changed, conversation_changed) = self
            .database
            .message_dao
            .mark_conversation_read(conversation_id, &self.account_id)
            .await?;
        if messages_changed {
            self.app_service.expired_message.wake();
            self.app_service
                .job
                .wake(sdk::ACKNOWLEDGE_MESSAGE_RECEIPTS)?;
            self.app_service
                .job
                .wake(sdk::blaze_message::CREATE_MESSAGE)?;
        }
        if messages_changed || conversation_changed {
            self.notify_conversation_changed(conversation_id);
        }
        Ok(())
    }

    async fn text_category(&self, owner_id: Option<&str>) -> Result<String> {
        let Some(owner_id) = owner_id else {
            return Ok(sdk::message_category::SIGNAL_TEXT.to_string());
        };
        if let Some(app) = self.database.app_dao.find_app_by_id(owner_id).await? {
            return Ok(if app
                .capabilities
                .is_some_and(|capabilities| capabilities.contains("ENCRYPTED"))
            {
                sdk::message_category::ENCRYPTED_TEXT
            } else {
                sdk::message_category::PLAIN_TEXT
            }
            .to_string());
        }
        let owner_ids = [owner_id.to_string()];
        let is_bot = self
            .database
            .user_dao
            .find_users(&owner_ids)
            .await?
            .first()
            .is_some_and(|owner| owner.app_id.is_some());
        Ok(if is_bot {
            sdk::message_category::PLAIN_TEXT
        } else {
            sdk::message_category::SIGNAL_TEXT
        }
        .to_string())
    }

    async fn messages_by_ids(&self, message_ids: &[String]) -> Result<Vec<Message>> {
        let messages = self
            .database
            .message_dao
            .find_messages_by_ids(message_ids)
            .await?;
        let by_id = messages
            .into_iter()
            .map(|message| (message.message_id.clone(), message))
            .collect::<HashMap<_, _>>();
        message_ids
            .iter()
            .map(|message_id| {
                by_id
                    .get(message_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("message not found: {message_id}"))
            })
            .collect()
    }
}

fn is_shareable_app_card_action(action: &str) -> bool {
    let Ok(uri) = url::Url::parse(action) else {
        return false;
    };
    if uri.scheme() == "mixin" && uri.host_str() == Some("send") {
        return uri
            .query_pairs()
            .any(|(key, value)| key == "user" && !value.trim().is_empty());
    }
    matches!(uri.scheme(), "http" | "https")
        && !(matches!(uri.host_str(), Some("mixin.one" | "www.mixin.one"))
            && uri.path().trim_matches('/').starts_with("send"))
}

fn normalized_quote_content(
    content: &str,
    is_transcript: bool,
    account_data_dir: impl FnOnce() -> Result<std::path::PathBuf>,
) -> Result<Option<String>> {
    let Ok(mut quote) = serde_json::from_str::<serde_json::Value>(content) else {
        return Ok(None);
    };
    let Some(quote) = quote.as_object_mut() else {
        return Ok(None);
    };
    let string = |key| quote.get(key).and_then(serde_json::Value::as_str);
    let Some(category) = string("type").map(str::to_string) else {
        return Ok(None);
    };
    let media_url = string("media_url");
    if media_url.is_some_and(|value| Path::new(value).is_absolute()) || !category.is_attachment() {
        return Ok(None);
    }
    let (Some(message_id), Some(conversation_id)) =
        (string("message_id"), string("conversation_id"))
    else {
        return Ok(None);
    };
    let message = Message {
        message_id: message_id.to_string(),
        conversation_id: conversation_id.to_string(),
        category,
        media_mime_type: string("media_mime_type").map(str::to_string),
        name: string("media_name").map(str::to_string),
        ..Message::default()
    };
    let account_data_dir = account_data_dir()?;
    let path = if is_transcript {
        transcript_attachment_path(&account_data_dir, &message)?
    } else {
        attachment_path(&account_data_dir, &message)?
    };
    quote.insert(
        "media_url".to_string(),
        serde_json::Value::String(path.to_string_lossy().into_owned()),
    );
    Ok(Some(serde_json::to_string(quote)?))
}

#[cfg(test)]
mod tests {
    use super::normalized_quote_content;
    use std::path::PathBuf;

    #[test]
    fn normalizes_quote_media_path_without_changing_stored_content() {
        let stored = r#"{"message_id":"quoted","conversation_id":"conversation","type":"PLAIN_IMAGE","media_url":"quoted.png","media_mime_type":"image/png"}"#;

        let normalized = normalized_quote_content(stored, false, || Ok(PathBuf::from("/account")))
            .unwrap()
            .unwrap();
        let normalized: serde_json::Value = serde_json::from_str(&normalized).unwrap();

        assert_eq!(
            normalized["media_url"],
            "/account/Media/Images/conversation/quoted.png"
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(stored).unwrap()["media_url"],
            "quoted.png"
        );
    }

    #[test]
    fn restores_quote_media_path_when_stored_content_predates_download() {
        let stored = r#"{"message_id":"quoted","conversation_id":"conversation","type":"PLAIN_IMAGE","media_mime_type":"image/png"}"#;

        let normalized = normalized_quote_content(stored, false, || Ok(PathBuf::from("/account")))
            .unwrap()
            .unwrap();
        let normalized: serde_json::Value = serde_json::from_str(&normalized).unwrap();

        assert_eq!(
            normalized["media_url"],
            "/account/Media/Images/conversation/quoted.png"
        );
    }
}
