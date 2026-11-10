mod actions;
mod attachment_send;
mod audio_send;
mod forwarding;
mod pinning;
mod remote_image;
mod transcript;

use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::{DateTime, Utc};
use log::warn;
use uuid::Uuid;

use sdk::message_category::MessageCategory as _;
use sdk::{ContactMessage, ConversationCategory, LiveMessage, MessageStatus, StickerMessage};

use crate::core::attachment::{attachment_path, transcript_attachment_path};
use crate::core::model::AttachmentExtra;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::{MediaStatus, Message};
use crate::db::path::account_data_directory;

use super::{forward_category, model, AccountState};

pub struct MessageAccess {
    state: Arc<AccountState>,
}

impl MessageAccess {
    pub(super) async fn ensure_can_send(&self, conversation_id: &str) -> Result<()> {
        if !self
            .database
            .conversation_dao
            .can_send(conversation_id, &self.account_id)
            .await?
        {
            return Err(crate::CoreError::PermissionDenied.into());
        }
        Ok(())
    }

    fn message_list_view(
        &self,
        item: crate::db::mixin::message::MessageListItem,
    ) -> Result<model::MessageListView> {
        let mut view: model::MessageListView = item.into();
        normalize_message_media_paths(
            &mut view,
            false,
            &account_data_directory(&self.profile.borrow().identity_number)?,
        )?;
        Ok(view)
    }

    fn transcript_message_list_view(
        &self,
        item: crate::db::mixin::transcript_message::TranscriptMessageListItem,
    ) -> Result<model::MessageListView> {
        let mut view: model::MessageListView = item.into();
        normalize_message_media_paths(
            &mut view,
            true,
            &account_data_directory(&self.profile.borrow().identity_number)?,
        )?;
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
        self.messages_in_range(
            conversation_id,
            before_created_at_micros,
            before_message_id,
            limit.clamp(1, 200),
            (None, None),
        )
        .await
    }

    pub(crate) async fn messages_in_range(
        &self,
        conversation_id: String,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<String>,
        limit: i64,
        range_millis: (Option<i64>, Option<i64>),
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
        self.database
            .message_dao
            .list_items_in_range(
                conversation_id,
                before_created_at_millis,
                before_message_id,
                limit,
                range_millis,
            )
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()
    }

    pub async fn message_items_by_ids(
        &self,
        message_ids: Vec<String>,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .message_dao
            .list_items_by_ids(&message_ids)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()
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
        self.ensure_active()?;
        if query.trim().is_empty() {
            let Some(sender_id) = sender_id else {
                return Ok(Vec::new());
            };
            return self
                .database
                .message_dao
                .list_items_by_sender(
                    &conversation_id,
                    &sender_id,
                    &categories,
                    anchor_message_id.as_deref(),
                    limit,
                )
                .await?
                .into_iter()
                .map(|item| self.message_list_view(item))
                .collect();
        }
        self.search_message_items(crate::db::mixin::message_fts::MessageFtsSearch {
            query,
            conversation_ids: vec![conversation_id],
            sender_ids: sender_id.into_iter().collect(),
            categories,
            cursor_message_id: anchor_message_id,
            limit,
            ..Default::default()
        })
        .await
    }

    pub async fn search_global_messages(
        &self,
        query: String,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>> {
        self.search_message_items(crate::db::mixin::message_fts::MessageFtsSearch {
            query,
            cursor_message_id: anchor_message_id,
            limit,
            ..Default::default()
        })
        .await
    }

    pub async fn search_scoped_messages(
        &self,
        query: String,
        category: String,
        circle_id: Option<String>,
        unseen_only: bool,
        anchor_message_id: Option<String>,
        limit: u32,
    ) -> Result<Vec<model::MessageListView>> {
        self.search_message_items(crate::db::mixin::message_fts::MessageFtsSearch {
            query,
            conversation_category: Some(category),
            circle_id,
            unseen_only,
            cursor_message_id: anchor_message_id,
            limit,
            ..Default::default()
        })
        .await
    }

    pub(crate) async fn search_message_items(
        &self,
        input: crate::db::mixin::message_fts::MessageFtsSearch,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        let matches = self.database.message_fts_dao.search(&input).await?;
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

    pub async fn messages_relative(
        &self,
        conversation_id: String,
        anchor_message_id: String,
        older: bool,
        limit: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .message_dao
            .list_items_relative(&conversation_id, &anchor_message_id, older, limit)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect()
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
        self.database
            .message_dao
            .list_items_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect::<Result<Vec<_>>>()
    }

    pub async fn first_unread_message_id(&self, conversation_id: String) -> Result<Option<String>> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .first_unread_message_id(&conversation_id, &self.account_id)
            .await?)
    }

    pub async fn audio_messages_around(
        &self,
        conversation_id: String,
        target_message_id: String,
        before: i64,
        after: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .message_dao
            .list_audio_items_around(&conversation_id, &target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.message_list_view(item))
            .collect()
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
        self.database
            .message_dao
            .list_image_items_around(conversation_id, target_message_id, before, after)
            .await?
            .into_iter()
            .map(|item| self.image_message_view(item, conversation_id))
            .collect::<Result<Vec<_>>>()
    }

    pub async fn transcript_messages(
        &self,
        transcript_id: String,
    ) -> Result<Vec<model::MessageListView>> {
        let transcript_id = transcript_id.as_str();
        self.ensure_active()?;
        self.database
            .transcript_message_dao
            .list_items(transcript_id)
            .await?
            .into_iter()
            .map(|item| self.transcript_message_list_view(item))
            .collect::<Result<Vec<_>>>()
    }

    /// Loads a bounded transcript window without retaining its entire history.
    pub async fn transcript_messages_page(
        &self,
        transcript_id: String,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<model::MessageListView>> {
        self.ensure_active()?;
        self.database
            .transcript_message_dao
            .list_items_page(&transcript_id, offset, limit)
            .await?
            .into_iter()
            .map(|item| self.transcript_message_list_view(item))
            .collect()
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
        self.ensure_can_send(conversation_id).await?;
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
            .await
            .map_err(crate::CoreError::from)?;
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(conversation_id);
        Ok(message_id)
    }

    pub(crate) async fn send_text_message(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
        post: bool,
        silent: bool,
    ) -> Result<String> {
        let form = if post { TextForm::Post } else { TextForm::Text };
        self.send_text_form(conversation_id, content, quote_message_id, form, silent)
            .await
    }

    pub async fn send_location(
        &self,
        conversation_id: String,
        latitude: f64,
        longitude: f64,
        name: Option<String>,
        address: Option<String>,
        quote_message_id: Option<String>,
    ) -> Result<String> {
        let content = location_content(&sdk::LocationMessage {
            latitude,
            longitude,
            name,
            address,
            venue_type: None,
        })?;
        self.send_text_form(
            conversation_id,
            content,
            quote_message_id,
            TextForm::Location,
            false,
        )
        .await
    }

    async fn send_text_form(
        &self,
        conversation_id: String,
        content: String,
        quote_message_id: Option<String>,
        form: TextForm,
        silent: bool,
    ) -> Result<String> {
        let quote_message_id = quote_message_id.as_deref();
        let conversation_id = conversation_id.as_str();
        let content = content.as_str();
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.ensure_can_send(conversation_id).await?;
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
        let mut recipient_id = None;
        match form {
            TextForm::Post => category = category.replace("_TEXT", "_POST"),
            TextForm::Location => category = category.replace("_TEXT", "_LOCATION"),
            TextForm::Text => {
                if let Some(bot) = bot_recipient(&self.database, conversation_id, content).await? {
                    category = bot.category.to_string();
                    recipient_id = Some(bot.app_id);
                }
            }
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
            recipient_id.as_deref(),
            None,
            false,
            silent,
            conversation.expire_in,
        );
        self.database
            .message_dao
            .insert_outgoing_message(&message, &job)
            .await
            .map_err(crate::CoreError::from)?;
        if !matches!(form, TextForm::Location) {
            if let Err(error) = self
                .database
                .message_fts_dao
                .upsert(&message_id, conversation_id, content)
                .await
            {
                warn!("failed to index outgoing message {message_id}: {error}");
            }
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
        self.ensure_can_send(conversation_id).await?;
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
            .await
            .map_err(crate::CoreError::from)?;
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
        self.ensure_can_send(conversation_id).await?;
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
            .await
            .map_err(crate::CoreError::from)?;
        self.database
            .sticker_dao
            .update_last_used(sticker_id)
            .await?;
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed(conversation_id);
        Ok(message_id)
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
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        let current_participant = self
            .database
            .participant_dao
            .find_participant_by_id(conversation_id, &self.account_id)
            .await?;
        let current_user_role = current_participant
            .as_ref()
            .and_then(|participant| participant.role.as_deref());
        let now = Utc::now().naive_utc();
        let messages = self.messages_by_ids(message_ids).await?;
        for (message, message_id) in messages.iter().zip(message_ids) {
            let sender_participant = self
                .database
                .participant_dao
                .find_participant_by_id(conversation_id, &message.user_id)
                .await?;
            if !can_recall_message(
                message,
                &conversation,
                &self.account_id,
                current_user_role,
                sender_participant.as_ref(),
                now,
            ) {
                return Err(anyhow!("message can not be recalled: {message_id}"));
            }
        }
        let jobs = message_ids
            .iter()
            .map(|message_id| Job::create_send_recall_job(conversation_id, message_id))
            .collect::<Vec<_>>();
        self.database
            .message_dao
            .insert_recall_jobs(conversation_id, message_ids, &jobs)
            .await?;
        self.app_service.job.wake(sdk::RECALL_MESSAGE)?;
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
        if self
            .database
            .message_mention_dao
            .mark_read_and_enqueue(conversation_id, Some(message_id))
            .await?
            == 0
        {
            return Ok(());
        }
        self.app_service.job.wake(sdk::CREATE_MESSAGE)?;
        self.notify_conversation_changed(conversation_id);
        Ok(())
    }

    pub async fn clear_unread_mentions(&self, conversation_id: String) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if self
            .database
            .message_mention_dao
            .mark_read_and_enqueue(&conversation_id, None)
            .await?
            > 0
        {
            self.app_service.job.wake(sdk::CREATE_MESSAGE)?;
            self.notify_conversation_changed(&conversation_id);
        }
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

fn validate_forward_source(message: &Message, target_prefix: &str) -> Result<()> {
    let message_id = &message.message_id;
    if !matches!(
        message.status,
        MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
    ) {
        return Err(anyhow!("message is not ready to forward: {message_id}"));
    }
    forward_category(&message.category, target_prefix)?;
    if message.category.is_transcript() && message.media_status != MediaStatus::Done {
        return Err(anyhow!("transcript is not downloaded: {message_id}"));
    }
    if message.category == sdk::message_category::APP_CARD {
        let shareable = message
            .content
            .as_deref()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
            .and_then(|value| value.get("shareable").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);
        if !shareable {
            return Err(anyhow!("app card is not shareable: {message_id}"));
        }
    } else if message.category.is_attachment() {
        if !matches!(message.media_status, MediaStatus::Done | MediaStatus::Read) {
            return Err(anyhow!("attachment is not available locally: {message_id}"));
        }
        let extra = message
            .content
            .as_deref()
            .and_then(|content| serde_json::from_str::<AttachmentExtra>(content).ok())
            .ok_or_else(|| anyhow!("attachment metadata is invalid: {message_id}"))?;
        if extra.shareable == Some(false) {
            return Err(anyhow!("attachment is not shareable: {message_id}"));
        }
        if message.media_url.as_deref().is_none_or(str::is_empty) {
            return Err(anyhow!("attachment has no local file: {message_id}"));
        }
        if let Some(waveform) = message.media_waveform.as_deref() {
            Base64::decode_vec(waveform)
                .with_context(|| format!("attachment waveform is invalid: {message_id}"))?;
        }
    } else if message.category.is_live() {
        let live = message
            .content
            .as_deref()
            .and_then(|content| serde_json::from_str::<LiveMessage>(content).ok())
            .ok_or_else(|| anyhow!("live message is invalid: {message_id}"))?;
        if !live.shareable {
            return Err(anyhow!("live message is not shareable: {message_id}"));
        }
    }
    Ok(())
}

fn can_recall_message(
    message: &Message,
    conversation: &crate::db::mixin::conversation::Conversation,
    current_user_id: &str,
    current_user_role: Option<&str>,
    sender_participant: Option<&crate::db::mixin::participant::Participant>,
    now: chrono::NaiveDateTime,
) -> bool {
    let sent = matches!(
        message.status,
        MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
    );
    if message.conversation_id != conversation.conversation_id
        || !sent
        || !message.category.can_recall()
        || now >= message.created_at + chrono::Duration::days(30)
    {
        return false;
    }
    if message.user_id == current_user_id {
        return true;
    }
    match conversation.category {
        Some(ConversationCategory::Contact) => true,
        Some(ConversationCategory::Group) => {
            if conversation.owner_id.as_deref() == Some(current_user_id)
                || current_user_role.is_some_and(|role| role.eq_ignore_ascii_case("OWNER"))
            {
                return true;
            }
            current_user_role.is_some_and(|role| role.eq_ignore_ascii_case("ADMIN"))
                && conversation.owner_id.as_deref() != Some(message.user_id.as_str())
                && sender_participant.is_some_and(|participant| participant.role.is_none())
        }
        None => false,
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

#[derive(Clone, Copy)]
enum TextForm {
    Text,
    Post,
    Location,
}

fn location_content(location: &sdk::LocationMessage) -> Result<String> {
    if !(-90.0..=90.0).contains(&location.latitude)
        || !(-180.0..=180.0).contains(&location.longitude)
    {
        return Err(anyhow!("location coordinates are out of range"));
    }
    Ok(serde_json::to_string(location)?)
}

struct BotRecipient {
    app_id: String,
    category: &'static str,
}

/// The number in a leading `@700... ` when a text is addressed to a bot.
fn bot_number(content: &str) -> Option<&str> {
    let rest = content.strip_prefix('@')?;
    let (number, _) = rest.split_once(' ')?;
    (number.starts_with("700") && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some(number)
}

/// A text starting with a participating bot's number goes only to that bot,
/// in the encryption it supports.
async fn bot_recipient(
    database: &crate::db::MixinDatabase,
    conversation_id: &str,
    content: &str,
) -> Result<Option<BotRecipient>> {
    let Some(number) = bot_number(content) else {
        return Ok(None);
    };
    let app = database
        .app_dao
        .find_participant_app_by_number(conversation_id, number)
        .await?;
    Ok(app.map(|app| BotRecipient {
        category: if app
            .capabilities
            .is_some_and(|capabilities| capabilities.contains("ENCRYPTED"))
        {
            sdk::message_category::ENCRYPTED_TEXT
        } else {
            sdk::message_category::PLAIN_TEXT
        },
        app_id: app.app_id,
    }))
}

pub(crate) fn normalize_message_media_paths(
    view: &mut model::MessageListView,
    is_transcript: bool,
    account_data_dir: &Path,
) -> Result<()> {
    let media_url = view.media_url.as_deref();
    if media_url.is_some_and(|value| {
        !value.trim().is_empty() && !Path::new(value).is_absolute() && view.category.is_attachment()
    }) {
        let message = Message {
            message_id: view.message_id.clone(),
            conversation_id: view.conversation_id.clone(),
            category: view.category.clone(),
            media_mime_type: view.media_mime_type.clone(),
            name: view.media_name.clone(),
            ..Message::default()
        };
        let path = if is_transcript {
            transcript_attachment_path(account_data_dir, &message)?
        } else {
            attachment_path(account_data_dir, &message)?
        };
        view.media_url = Some(path.to_string_lossy().into_owned());
    }
    if let Some(content) = view.quote_content.as_deref() {
        if let Some(content) =
            normalized_quote_content(
                content,
                is_transcript,
                || Ok(account_data_dir.to_path_buf()),
            )?
        {
            view.quote_content = Some(content);
        }
    }
    Ok(())
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
mod location_tests {
    use super::location_content;

    fn location(latitude: f64, longitude: f64) -> sdk::LocationMessage {
        sdk::LocationMessage {
            latitude,
            longitude,
            name: Some("Cafe".into()),
            address: None,
            venue_type: None,
        }
    }

    #[test]
    fn valid_coordinates_produce_the_wire_payload_and_invalid_ones_are_rejected() {
        let content = location_content(&location(31.2, -121.5)).unwrap();
        let wire: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(wire["latitude"], 31.2);
        assert_eq!(wire["longitude"], -121.5);
        assert_eq!(wire["name"], "Cafe");

        for (latitude, longitude) in [(90.5, 0.0), (0.0, -180.5), (f64::NAN, 0.0)] {
            assert!(location_content(&location(latitude, longitude)).is_err());
        }
    }
}

#[cfg(test)]
mod bot_recipient_tests {
    use super::bot_recipient;

    #[tokio::test]
    async fn leading_number_of_a_participating_bot_addresses_only_that_bot() {
        let directory = tempfile::tempdir().unwrap();
        let database = crate::db::MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let pool = &database.user_dao.0;
        crate::db::query(
            "INSERT INTO conversations (conversation_id, category, name, created_at, status, \
             mute_until) VALUES ('group', 'GROUP', '', 0, 0, 0)",
        )
        .execute(pool)
        .await
        .unwrap();
        for (app, number, capabilities, joined) in [
            ("encrypted", "7000101", r#"["ENCRYPTED"]"#, true),
            ("plain", "7000102", r#"["CONTACT"]"#, true),
            ("absent", "7000103", r#"["CONTACT"]"#, false),
        ] {
            crate::db::query(
                "INSERT INTO apps (app_id, app_number, home_uri, redirect_uri, name, icon_url, \
                 category, description, app_secret, capabilities, creator_id, updated_at) \
                 VALUES (?, ?, '', '', '', '', '', '', '', ?, '', 0)",
            )
            .bind(app)
            .bind(number)
            .bind(capabilities)
            .execute(pool)
            .await
            .unwrap();
            if joined {
                crate::db::query(
                    "INSERT INTO participants (conversation_id, user_id, created_at) \
                     VALUES ('group', ?, 0)",
                )
                .bind(app)
                .execute(pool)
                .await
                .unwrap();
            }
        }

        for (content, expected) in [
            ("@7000101 hello", Some(("encrypted", "ENCRYPTED_TEXT"))),
            ("@7000102 hello", Some(("plain", "PLAIN_TEXT"))),
            ("@7000103 hello", None),
            ("@7000102", None),
            ("hello @7000102 there", None),
            ("@7000102x hello", None),
        ] {
            let found = bot_recipient(&database, "group", content).await.unwrap();
            assert_eq!(
                found
                    .as_ref()
                    .map(|bot| (bot.app_id.as_str(), bot.category)),
                expected,
                "{content}"
            );
        }
        database.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::{can_recall_message, normalized_quote_content, validate_forward_source};
    use std::path::PathBuf;

    use chrono::{Duration, Utc};
    use sdk::{ConversationCategory, MessageStatus};

    use crate::db::mixin::conversation::{Conversation, ConversationStatus};
    use crate::db::mixin::message::{MediaStatus, Message};
    use crate::db::mixin::participant::Participant;

    #[test]
    fn forwarding_transcript_requires_completed_media_and_combining_remains_disallowed() {
        let mut message = Message {
            category: "SIGNAL_TRANSCRIPT".into(),
            status: MessageStatus::Sent,
            ..Default::default()
        };
        for status in [MediaStatus::Pending, MediaStatus::Canceled] {
            message.media_status = status;
            assert!(validate_forward_source(&message, "PLAIN").is_err());
        }
        message.media_status = MediaStatus::Done;
        assert!(validate_forward_source(&message, "PLAIN").is_ok());
        assert!(crate::runtime::validate_combine_forward_source(&message).is_err());
    }

    #[test]
    fn forwarding_rejects_unsent_or_unshareable_messages() {
        let mut message = recallable_message(Utc::now().naive_utc());
        assert!(validate_forward_source(&message, "SIGNAL").is_ok());
        message.status = MessageStatus::Sending;
        assert!(validate_forward_source(&message, "SIGNAL").is_err());
        message.status = MessageStatus::Sent;
        message.category = sdk::message_category::APP_CARD.into();
        for content in [None, Some("invalid"), Some(r#"{"shareable":false}"#)] {
            message.content = content.map(str::to_owned);
            assert!(validate_forward_source(&message, "SIGNAL").is_err());
        }
        message.content = Some(r#"{"shareable":true}"#.into());
        assert!(validate_forward_source(&message, "SIGNAL").is_ok());
    }

    #[test]
    fn forwarding_requires_downloaded_shareable_attachment_metadata_and_valid_waveform() {
        use crate::db::mixin::message::MediaStatus;
        let mut message = recallable_message(Utc::now().naive_utc());
        message.category = "SIGNAL_IMAGE".into();
        message.media_status = MediaStatus::Done;
        message.media_url = Some("image.png".into());
        message.content = Some(r#"{"attachment_id":"attachment","message_id":"message"}"#.into());
        assert!(validate_forward_source(&message, "SIGNAL").is_ok());
        message.media_status = MediaStatus::Pending;
        assert!(validate_forward_source(&message, "SIGNAL").is_err());
        message.media_status = MediaStatus::Done;
        message.content = Some(
            r#"{"attachment_id":"attachment","message_id":"message","shareable":false}"#.into(),
        );
        assert!(validate_forward_source(&message, "SIGNAL").is_err());
        message.content = Some(r#"{"attachment_id":"attachment","message_id":"message"}"#.into());
        message.media_waveform = Some("invalid base64!".into());
        assert!(validate_forward_source(&message, "SIGNAL").is_err());
        message.media_waveform = None;
        message.media_url = None;
        assert!(validate_forward_source(&message, "SIGNAL").is_err());
    }

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

    #[test]
    fn recall_policy_allows_direct_peers_and_group_role_hierarchy() {
        let now = Utc::now().naive_utc();
        let mut conversation = conversation(ConversationCategory::Contact);
        let mut message = recallable_message(now - Duration::days(1));

        assert!(can_recall_message(
            &message,
            &conversation,
            "current",
            None,
            None,
            now,
        ));

        conversation.category = Some(ConversationCategory::Group);
        let regular = participant("sender", None);
        assert!(can_recall_message(
            &message,
            &conversation,
            "current",
            Some("OWNER"),
            Some(&regular),
            now,
        ));
        assert!(can_recall_message(
            &message,
            &conversation,
            "current",
            Some("ADMIN"),
            Some(&regular),
            now,
        ));

        let admin = participant("sender", Some("ADMIN"));
        assert!(!can_recall_message(
            &message,
            &conversation,
            "current",
            Some("ADMIN"),
            Some(&admin),
            now,
        ));
        assert!(!can_recall_message(
            &message,
            &conversation,
            "current",
            Some("ADMIN"),
            None,
            now,
        ));

        message.user_id = "owner".to_string();
        assert!(!can_recall_message(
            &message,
            &conversation,
            "current",
            Some("ADMIN"),
            Some(&regular),
            now,
        ));
    }

    #[test]
    fn recall_policy_keeps_status_category_and_thirty_day_constraints() {
        let now = Utc::now().naive_utc();
        let conversation = conversation(ConversationCategory::Contact);
        let mut message = recallable_message(now - Duration::days(29));
        message.user_id = "current".to_string();

        assert!(can_recall_message(
            &message,
            &conversation,
            "current",
            None,
            None,
            now,
        ));
        message.created_at = now - Duration::days(30);
        assert!(!can_recall_message(
            &message,
            &conversation,
            "current",
            None,
            None,
            now,
        ));
        message.created_at = now - Duration::days(1);
        message.status = MessageStatus::Sending;
        assert!(!can_recall_message(
            &message,
            &conversation,
            "current",
            None,
            None,
            now,
        ));
        message.status = MessageStatus::Read;
        message.category = "SYSTEM_USER".to_string();
        assert!(!can_recall_message(
            &message,
            &conversation,
            "current",
            None,
            None,
            now,
        ));
    }

    fn conversation(category: ConversationCategory) -> Conversation {
        Conversation {
            conversation_id: "conversation".to_string(),
            owner_id: Some("owner".to_string()),
            category: Some(category),
            name: String::new(),
            icon_url: String::new(),
            announcement: String::new(),
            code_url: String::new(),
            created_at: Utc::now(),
            status: ConversationStatus::SUCCESS,
            mute_until: Utc::now(),
            expire_in: 0,
        }
    }

    fn recallable_message(created_at: chrono::NaiveDateTime) -> Message {
        Message {
            message_id: "message".to_string(),
            conversation_id: "conversation".to_string(),
            user_id: "sender".to_string(),
            category: "PLAIN_TEXT".to_string(),
            status: MessageStatus::Read,
            created_at,
            ..Message::default()
        }
    }

    fn participant(user_id: &str, role: Option<&str>) -> Participant {
        Participant {
            conversation_id: "conversation".to_string(),
            user_id: user_id.to_string(),
            role: role.map(str::to_string),
            created_at: Utc::now(),
        }
    }
}
