use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use base64ct::{Base64, Encoding};
use chrono::Utc;
use log::{error, warn};
use tokio::sync::{oneshot, watch, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use sdk::message_category::MessageCategory as _;
use sdk::{
    Account, AttachmentMessage, CircleConversationRequest, Client, ContactMessage,
    ConversationCategory, ConversationRequest, Credential, KeyStore, LiveMessage, MessageStatus,
    ParticipantRequest, PinMessagePayload, RelationshipAction, StickerMessage,
};

use crate::core::attachment::AttachmentService;
use crate::core::constants::SCP;
use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::message::blaze::Blaze;
use crate::core::message::decrypt::{
    transcript_attachment_id, transcript_attachment_message, ServiceDecryptMessage,
};
use crate::core::message::sender::MessageSender;
use crate::core::model::job::sanitize_transcript_app_card;
use crate::core::model::signal::SignalService;
use crate::core::model::{AppService, AttachmentExtra, ConversationService};
use crate::db::app::Auth;
use crate::db::mixin::circle::Circle;
use crate::db::mixin::conversation::ConversationListItem;
use crate::db::mixin::job::Job;
use crate::db::mixin::message::{ImageMessageItem, MediaStatus, Message, MessageListItem};
use crate::db::mixin::transcript_message::{TranscriptMessage, TranscriptMessageListItem};
use crate::db::mixin::user::User;
use crate::db::path::account_data_directory;
use crate::db::{MixinDatabase, SignalDatabase};

type AccountStartupResult =
    std::result::Result<(Arc<MixinDatabase>, Arc<SignalDatabase>, Arc<AppService>), String>;

const MIN_STICKER_FILE_SIZE: usize = 1024;
const MAX_STICKER_FILE_SIZE: usize = 1024 * 1024;
const MIN_STICKER_DIMENSION: u32 = 128;
const MAX_STICKER_DIMENSION: u32 = 1024;

pub struct AccountRuntime {
    account_id: String,
    account: Account,
    client: Arc<Client>,
    database: Arc<MixinDatabase>,
    signal_database: Arc<SignalDatabase>,
    app_service: Arc<AppService>,
    conversation_changes: watch::Sender<u64>,
    shutdown: watch::Sender<bool>,
    active: AtomicBool,
    mutation_gate: RwLock<()>,
    attachment_downloads: Mutex<HashMap<String, CancellationToken>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl AccountRuntime {
    pub async fn start(auth: Auth) -> Result<Self> {
        if auth
            .primary_session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .is_none()
        {
            return Err(anyhow!("authorization has no primary session"));
        }
        let account_id = auth.account.user_id.clone();
        let account = auth.account.clone();
        let client = Arc::new(Client::new(credential(&auth)));
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let (conversation_changes, _) = watch::channel(0);
        let account_conversation_changes = conversation_changes.clone();
        let (ready_sender, ready_receiver) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name(format!("mixin-account-{account_id}"))
            .spawn({
                let client = client.clone();
                move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    match runtime {
                        Ok(runtime) => runtime.block_on(run_account(
                            auth,
                            client,
                            shutdown_receiver,
                            account_conversation_changes,
                            ready_sender,
                        )),
                        Err(error) => {
                            let _ = ready_sender.send(Err(error.to_string()));
                        }
                    }
                }
            })?;
        let (database, signal_database, app_service) = ready_receiver
            .await
            .map_err(|_| anyhow!("account runtime stopped during startup"))?
            .map_err(|error| anyhow!(error))?;

        Ok(Self {
            account_id,
            account,
            client,
            database,
            signal_database,
            app_service,
            conversation_changes,
            shutdown,
            active: AtomicBool::new(true),
            mutation_gate: RwLock::new(()),
            attachment_downloads: Mutex::new(HashMap::new()),
            thread: Mutex::new(Some(thread)),
        })
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub async fn conversation_count(
        &self,
        category: &str,
        circle_id: Option<&str>,
        keyword: &str,
        unseen_only: bool,
    ) -> Result<i64> {
        Ok(self
            .database
            .conversation_dao
            .count_items(category, circle_id, keyword, unseen_only)
            .await?)
    }

    pub async fn conversations(
        &self,
        category: &str,
        circle_id: Option<&str>,
        keyword: &str,
        unseen_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConversationListItem>> {
        Ok(self
            .database
            .conversation_dao
            .list_items(category, circle_id, keyword, unseen_only, limit, offset)
            .await?)
    }

    pub async fn messages(
        &self,
        conversation_id: &str,
        before_created_at_micros: Option<i64>,
        before_message_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MessageListItem>> {
        if before_created_at_micros.is_some() != before_message_id.is_some() {
            return Err(anyhow!(
                "message cursor requires both timestamp and message id"
            ));
        }
        let before_created_at = before_created_at_micros
            .map(|value| {
                chrono::DateTime::from_timestamp_micros(value)
                    .map(|date_time| date_time.naive_utc())
                    .ok_or_else(|| anyhow!("invalid message cursor timestamp: {value}"))
            })
            .transpose()?;
        Ok(self
            .database
            .message_dao
            .list_items(conversation_id, before_created_at, before_message_id, limit)
            .await?)
    }

    pub async fn current_user_role(&self, conversation_id: &str) -> Result<Option<String>> {
        let conversation = self
            .database
            .conversation_dao
            .find_conversation_by_id(conversation_id)
            .await?
            .ok_or_else(|| anyhow!("conversation not found: {conversation_id}"))?;
        if conversation.category != Some(ConversationCategory::Group) {
            return Ok(Some("OWNER".to_string()));
        }
        Ok(self
            .database
            .participant_dao
            .find_participant_by_id(conversation_id, &self.account_id)
            .await?
            .and_then(|participant| participant.role))
    }

    pub async fn user_profile(
        &self,
        user_id: Option<&str>,
        identity_number: Option<&str>,
    ) -> Result<Option<User>> {
        if user_id.is_none() && identity_number.is_none() {
            return Err(anyhow!("user id or identity number is required"));
        }
        if let Some(user_id) = user_id {
            if let Some(user) = self.database.user_dao.find_user_by_id(user_id).await? {
                return Ok(Some(user));
            }
        }
        if let Some(identity_number) = identity_number {
            return Ok(self
                .database
                .user_dao
                .find_user_by_identity_number(identity_number)
                .await?);
        }
        Ok(None)
    }

    pub async fn users_by_identity_numbers(
        &self,
        identity_numbers: &[String],
    ) -> Result<Vec<User>> {
        Ok(self
            .database
            .user_dao
            .find_users_by_identity_numbers(identity_numbers)
            .await?)
    }

    pub async fn messages_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageListItem>> {
        Ok(self
            .database
            .message_dao
            .list_items_around(conversation_id, target_message_id, before, after)
            .await?)
    }

    pub async fn image_messages_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<ImageMessageItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .list_image_items_around(conversation_id, target_message_id, before, after)
            .await?)
    }

    pub async fn pinned_messages(&self, conversation_id: &str) -> Result<Vec<MessageListItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .message_dao
            .list_pinned_items(conversation_id)
            .await?)
    }

    pub async fn transcript_messages(
        &self,
        transcript_id: &str,
    ) -> Result<Vec<TranscriptMessageListItem>> {
        self.ensure_active()?;
        Ok(self
            .database
            .transcript_message_dao
            .list_items(transcript_id)
            .await?)
    }

    pub async fn download_transcript_attachment(
        &self,
        transcript_id: &str,
        message_id: &str,
    ) -> Result<()> {
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
        transcript_id: &str,
        message_id: &str,
    ) -> Result<()> {
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
        transcript_id: &str,
        message_id: &str,
    ) -> Result<()> {
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

    pub async fn download_attachment(&self, message_id: &str) -> Result<()> {
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

    pub async fn cancel_attachment(&self, message_id: &str) -> Result<()> {
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

    pub async fn mark_audio_read(&self, message_id: &str) -> Result<()> {
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

    pub async fn add_sticker(&self, sticker_id: &str) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        if sticker_id.trim().is_empty() {
            return Err(anyhow!("sticker id is required"));
        }
        let sticker = self.client.account_api.add_sticker(sticker_id).await?;
        self.database.sticker_dao.insert(&sticker).await?;
        if let Some(album_id) = sticker.album_id.as_deref().filter(|id| !id.is_empty()) {
            self.database
                .sticker_dao
                .insert_relationship(album_id, &sticker.sticker_id)
                .await?;
        }
        Ok(())
    }

    pub async fn add_sticker_from_file(&self, message_id: &str) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let message = self
            .database
            .message_dao
            .find_message_by_id(&message_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("image message not found: {message_id}"))?;
        if !message.category.is_image() {
            return Err(anyhow!("message is not an image: {message_id}"));
        }
        if !matches!(message.media_status, MediaStatus::Done | MediaStatus::Read) {
            return Err(anyhow!("image is not available locally: {message_id}"));
        }
        let path = Path::new(
            message
                .media_url
                .as_deref()
                .filter(|path| !path.is_empty())
                .ok_or_else(|| anyhow!("image has no local file: {message_id}"))?,
        );
        let bytes = self
            .app_service
            .attachment
            .read_account_file(path, MAX_STICKER_FILE_SIZE as u64)
            .await?;
        validate_sticker_image(path, &bytes)?;

        let personal_album_id = self.database.sticker_dao.find_personal_album_id().await?;
        let encoded = Base64::encode_string(&bytes);
        let sticker = self.client.account_api.add_sticker_data(&encoded).await?;
        let album_id = sticker
            .album_id
            .clone()
            .filter(|album_id| !album_id.is_empty())
            .or(personal_album_id);
        self.database.sticker_dao.insert(&sticker).await?;
        if let Some(album_id) = album_id {
            self.database
                .sticker_dao
                .insert_relationship(&album_id, &sticker.sticker_id)
                .await?;
        }
        Ok(())
    }

    pub async fn add_contact(&self, user_id: &str, full_name: &str) -> Result<()> {
        self.update_relationship(RelationshipAction::Add {
            user_id: user_id.to_string(),
            full_name: full_name.to_string(),
        })
        .await
    }

    pub async fn block_user(&self, user_id: &str) -> Result<()> {
        self.update_relationship(RelationshipAction::Block {
            user_id: user_id.to_string(),
        })
        .await
    }

    async fn update_relationship(&self, action: RelationshipAction) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user = self.client.user_api.update_relationship(&action).await?;
        self.database.user_dao.insert_sdk_users(vec![user]).await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn bot_home_uri(&self, app_id: &str) -> Result<Option<String>> {
        self.ensure_active()?;
        Ok(self
            .database
            .app_dao
            .find_app_by_id(app_id)
            .await?
            .map(|app| app.home_uri)
            .filter(|uri| !uri.trim().is_empty()))
    }

    pub async fn send_text(
        &self,
        conversation_id: &str,
        content: &str,
        quote_message_id: Option<&str>,
    ) -> Result<String> {
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
        let category = self.text_category(conversation.owner_id.as_deref()).await?;
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
            false,
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
        self.notify_conversation_changed();
        Ok(message_id)
    }

    pub async fn forward_messages(
        &self,
        target_conversation_id: &str,
        source_message_ids: &[String],
    ) -> Result<Vec<String>> {
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
                let source_path = Path::new(source.media_url.as_deref().unwrap_or_default());
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
                    .copy_for_forward(source_path, &target_file_message)
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
                            .upload(&target_path, target_prefix != "PLAIN")
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
                media_url = Some(target_path.to_string_lossy().into_owned());
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
                    name: String::new(),
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
        self.notify_conversation_changed();
        Ok(forwarded_ids)
    }

    pub async fn combine_forward_messages(
        &self,
        target_conversation_id: &str,
        source_message_ids: &[String],
    ) -> Result<String> {
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
            validate_combine_forward_source(&source)?;
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
        self.app_service.job.wake(&job.action)?;
        self.notify_conversation_changed();
        Ok(transcript_id)
    }

    pub async fn delete_messages(
        &self,
        conversation_id: &str,
        message_ids: &[String],
    ) -> Result<()> {
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
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn recall_messages(
        &self,
        conversation_id: &str,
        message_ids: &[String],
    ) -> Result<()> {
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
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn set_message_pinned(
        &self,
        conversation_id: &str,
        message_id: &str,
        pinned: bool,
    ) -> Result<()> {
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
        self.database
            .message_dao
            .set_message_pinned_with_job(conversation_id, message_id, pinned, Utc::now(), &job)
            .await?;
        self.app_service.job.wake(sdk::PIN_MESSAGE)?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn mark_mention_read(&self, conversation_id: &str, message_id: &str) -> Result<()> {
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
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn mark_conversation_read(&self, conversation_id: &str) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let changed = self
            .database
            .message_dao
            .mark_conversation_read(conversation_id, &self.account_id)
            .await?;
        if !changed {
            return Ok(());
        }
        self.app_service.expired_message.wake();
        self.app_service
            .job
            .wake(sdk::ACKNOWLEDGE_MESSAGE_RECEIPTS)?;
        self.app_service
            .job
            .wake(sdk::blaze_message::CREATE_MESSAGE)?;
        self.notify_conversation_changed();
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

    pub async fn circles(&self) -> Result<Vec<Circle>> {
        Ok(self.database.circle_dao.list().await?)
    }

    pub fn subscribe_conversation_changes(&self) -> watch::Receiver<u64> {
        self.conversation_changes.subscribe()
    }

    pub fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    fn notify_conversation_changed(&self) {
        self.conversation_changes
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }

    fn ensure_active(&self) -> Result<()> {
        if self.active.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(anyhow!("account runtime is shutting down"))
        }
    }

    pub async fn set_pinned(&self, conversation_id: &str, pinned: bool) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .conversation_dao
            .set_pinned(conversation_id, pinned)
            .await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn set_muted(
        &self,
        conversation_id: &str,
        owner_id: &str,
        category: &str,
        duration_seconds: i64,
    ) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let category = if category == "GROUP" {
            ConversationCategory::Group
        } else {
            ConversationCategory::Contact
        };
        let response = self
            .client
            .conversation_api
            .mute(&ConversationRequest {
                conversation_id: conversation_id.to_string(),
                category: Some(category.clone()),
                name: None,
                icon_base64: None,
                announcement: None,
                participants: (category == ConversationCategory::Contact).then(|| {
                    vec![ParticipantRequest {
                        user_id: self.account_id.clone(),
                    }]
                }),
                duration: Some(duration_seconds),
            })
            .await?;
        self.database
            .conversation_dao
            .set_mute_until(
                conversation_id,
                owner_id,
                if category == ConversationCategory::Group {
                    "GROUP"
                } else {
                    "CONTACT"
                },
                response.mute_until,
            )
            .await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        self.database
            .conversation_dao
            .delete_local(conversation_id)
            .await?;
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn edit_circle_conversation(
        &self,
        circle_id: &str,
        conversation_id: &str,
        owner_id: &str,
        is_group: bool,
        add: bool,
    ) -> Result<()> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let user_id = (!is_group).then(|| owner_id.to_string());
        let request = if add {
            CircleConversationRequest::Add {
                conversation_id: conversation_id.to_string(),
                user_id,
            }
        } else {
            CircleConversationRequest::Remove {
                conversation_id: conversation_id.to_string(),
                user_id,
            }
        };
        let result = self
            .client
            .circle_api
            .update_circle_conversation(circle_id, &request)
            .await?;
        if add {
            self.database
                .circle_conversation_dao
                .insert(&[result])
                .await?;
        } else {
            self.database
                .circle_conversation_dao
                .delete(circle_id, conversation_id)
                .await?;
        }
        self.notify_conversation_changed();
        Ok(())
    }

    pub async fn shutdown(&self) {
        let _mutation = self.mutation_gate.write().await;
        self.shutdown_inner().await;
    }

    async fn shutdown_inner(&self) {
        self.active.store(false, Ordering::Release);
        self.cancel_attachment_downloads();
        let _ = self.shutdown.send(true);
        let thread = self.thread.lock().unwrap().take();
        if let Some(thread) = thread {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
    }

    pub async fn revoke_session(&self) {
        if let Err(error) = self
            .client
            .account_api
            .logout(&self.account.session_id)
            .await
        {
            warn!("failed to revoke account session during sign out: {error}");
        }
    }

    pub fn begin_sign_out(&self) {
        self.active.store(false, Ordering::Release);
        self.cancel_attachment_downloads();
    }

    pub async fn sign_out(&self) {
        let _mutation = self.mutation_gate.write().await;
        self.shutdown_inner().await;
        self.revoke_session().await;
        if let Err(error) = self.signal_database.clear().await {
            warn!("failed to clear signal state during sign out: {error}");
        }
        if let Err(error) = self
            .database
            .participant_session_dao
            .clear_for_sign_out(&self.account.session_id)
            .await
        {
            warn!("failed to clear participant sessions during sign out: {error}");
        }
    }

    fn cancel_attachment_downloads(&self) {
        let downloads = self
            .attachment_downloads
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for cancellation in downloads.values() {
            cancellation.cancel();
        }
    }
}

impl Drop for AccountRuntime {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        self.cancel_attachment_downloads();
        let _ = self.shutdown.send(true);
    }
}

async fn run_account(
    auth: Auth,
    client: Arc<Client>,
    mut shutdown_receiver: watch::Receiver<bool>,
    conversation_changes: watch::Sender<u64>,
    ready_sender: oneshot::Sender<AccountStartupResult>,
) {
    let result = prepare_account(&auth, client, conversation_changes).await;
    let (database, signal_database, blaze, decrypt_message, sender, app_service) = match result {
        Ok(services) => services,
        Err(error) => {
            let _ = ready_sender.send(Err(error.to_string()));
            return;
        }
    };
    if ready_sender
        .send(Ok((database, signal_database, app_service.clone())))
        .is_err()
    {
        return;
    }

    tokio::select! {
        result = blaze.connect() => {
            if let Err(error) = result {
                error!("Blaze stopped: {error:?}");
            }
        }
        _ = decrypt_message.start() => {
            warn!("message decrypt service stopped");
        }
        _ = sender.maintain_signal_keys() => {
            warn!("signal key service stopped");
        }
        result = app_service.job.start() => {
            if let Err(error) = result {
                error!("job service stopped: {error:?}");
            }
        }
        _ = shutdown_receiver.changed() => {}
    }
}

type AccountServices = (
    Arc<MixinDatabase>,
    Arc<SignalDatabase>,
    Arc<Blaze>,
    Arc<ServiceDecryptMessage>,
    Arc<MessageSender>,
    Arc<AppService>,
);

async fn prepare_account(
    auth: &Auth,
    client: Arc<Client>,
    conversation_changes: watch::Sender<u64>,
) -> Result<AccountServices> {
    let account = &auth.account;
    let account_id = account.user_id.clone();
    let credential = credential(auth);
    client.account_api.get_me().await?;

    let database = Arc::new(
        MixinDatabase::new(account.identity_number.clone())
            .await
            .map_err(|error| anyhow!(error.to_string()))?,
    );
    let signal_database = Arc::new(
        SignalDatabase::connect(account.identity_number.clone())
            .await
            .map_err(|error| anyhow!(error.to_string()))?,
    );
    let blaze = Arc::new(Blaze::new(
        database.clone(),
        client.clone(),
        credential,
        account_id.clone(),
        Some(conversation_changes.clone()),
    ));
    let signal_protocol = Arc::new(SignalProtocol::new(
        signal_database.clone(),
        account_id.clone(),
    ));
    let conversation =
        ConversationService::new(database.clone(), client.clone(), account_id.clone());
    let signal_service = SignalService::new(signal_protocol.clone(), signal_database.clone());
    let sender = Arc::new(MessageSender::new(
        blaze.clone(),
        conversation,
        database.clone(),
        account_id.clone(),
        account.session_id.clone(),
        signal_protocol.clone(),
        signal_service,
    ));
    let attachment = Arc::new(AttachmentService::new(
        client.clone(),
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(150))
            .build()?,
        account_data_directory(&account.identity_number)?,
    ));
    let app_service = Arc::new(AppService::new(
        database.clone(),
        client.clone(),
        auth,
        sender.clone(),
        attachment,
        Some(conversation_changes.clone()),
    ));
    let decrypt_message = Arc::new(
        ServiceDecryptMessage::new(
            database.clone(),
            app_service.clone(),
            signal_protocol,
            sender.clone(),
            blaze.pending_message_statuses(),
            auth,
        )
        .with_conversation_changes(conversation_changes),
    );
    Ok((
        database,
        signal_database,
        blaze,
        decrypt_message,
        sender,
        app_service,
    ))
}

fn credential(auth: &Auth) -> Credential {
    Credential::KeyStore(KeyStore {
        app_id: auth.account.user_id.clone(),
        session_id: auth.account.session_id.clone(),
        server_public_key: String::new(),
        session_private_key: base16ct::lower::encode_string(&auth.private_key),
        scp: SCP.to_string(),
    })
}

fn transcript_download_key(transcript_id: &str, message_id: &str) -> String {
    format!("transcript:{transcript_id}:{message_id}")
}

fn validate_sticker_image(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.len() < MIN_STICKER_FILE_SIZE {
        return Err(anyhow!("sticker image must be at least 1KB"));
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|extension| matches!(extension.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp"))
        .ok_or_else(|| anyhow!("unsupported sticker image format"))?;
    let format = image::guess_format(bytes)?;
    if !matches!(
        (format, extension.as_str()),
        (image::ImageFormat::Jpeg, "jpg" | "jpeg")
            | (image::ImageFormat::Png, "png")
            | (image::ImageFormat::Gif, "gif")
            | (image::ImageFormat::WebP, "webp")
    ) {
        return Err(anyhow!(
            "sticker image format does not match its file extension"
        ));
    }
    let (width, height) = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .into_dimensions()?;
    if width.min(height) < MIN_STICKER_DIMENSION || width.max(height) > MAX_STICKER_DIMENSION {
        return Err(anyhow!(
            "sticker dimensions must be between 128 and 1024 pixels"
        ));
    }
    Ok(())
}

fn forward_category(source: &str, target_prefix: &str) -> Result<String> {
    if source == sdk::message_category::APP_CARD {
        return Ok(source.to_string());
    }
    let category = source.to_string();
    let suffix = if category.is_text() {
        "TEXT"
    } else if category.is_image() {
        "IMAGE"
    } else if category.is_video() {
        "VIDEO"
    } else if category.is_audio() {
        "AUDIO"
    } else if category.is_data() {
        "DATA"
    } else if category.is_post() {
        "POST"
    } else if category.is_contact() {
        "CONTACT"
    } else if category.is_location() {
        "LOCATION"
    } else if category.is_sticker() {
        "STICKER"
    } else if category.is_live() {
        "LIVE"
    } else if category.is_transcript() {
        "TRANSCRIPT"
    } else {
        return Err(anyhow!("message category cannot be forwarded: {source}"));
    };
    Ok(format!("{target_prefix}_{suffix}"))
}

fn validate_combine_forward_source(source: &Message) -> Result<()> {
    if !matches!(
        source.status,
        MessageStatus::Sent | MessageStatus::Delivered | MessageStatus::Read
    ) {
        return Err(anyhow!(
            "message is not ready to combine forward: {}",
            source.message_id
        ));
    }
    if source.category.is_transcript() {
        return Err(anyhow!(
            "transcript messages cannot be combined: {}",
            source.message_id
        ));
    }
    if source.category.is_attachment() {
        if !matches!(source.media_status, MediaStatus::Done | MediaStatus::Read) {
            return Err(anyhow!(
                "attachment is not ready to combine forward: {}",
                source.message_id
            ));
        }
        let content = source.content.as_deref().ok_or_else(|| {
            anyhow!(
                "attachment has no metadata for combine forward: {}",
                source.message_id
            )
        })?;
        let extra: AttachmentExtra = serde_json::from_str(content).map_err(|error| {
            anyhow!(
                "invalid attachment metadata for combine forward {}: {error}",
                source.message_id
            )
        })?;
        if source.category.is_audio() && extra.shareable == Some(false) {
            return Err(anyhow!(
                "audio attachment is not shareable: {}",
                source.message_id
            ));
        }
    }
    if source.category == sdk::message_category::APP_CARD {
        let shareable = source
            .content
            .as_deref()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
            .and_then(|value| value.get("shareable").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);
        if !shareable {
            return Err(anyhow!("app card is not shareable: {}", source.message_id));
        }
    }
    Ok(())
}

fn combine_transcript_category(source: &str, target_prefix: &str) -> Result<String> {
    if source == sdk::message_category::APP_CARD {
        return Ok(source.to_string());
    }
    let category = source.to_string();
    let suffix = if category.is_text() {
        "TEXT"
    } else if category.is_image() {
        "IMAGE"
    } else if category.is_video() {
        "VIDEO"
    } else if category.is_audio() {
        "AUDIO"
    } else if category.is_data() {
        "DATA"
    } else if category.is_sticker() {
        "STICKER"
    } else if category.is_contact() {
        "CONTACT"
    } else if category.is_live() {
        "LIVE"
    } else if category.is_post() {
        "POST"
    } else if category.is_location() {
        "LOCATION"
    } else {
        return Err(anyhow!("message category cannot be combined: {source}"));
    };
    Ok(format!("{target_prefix}_{suffix}"))
}

fn forward_transcript_category(source: &str, target_prefix: &str) -> Result<String> {
    if source == sdk::message_category::APP_CARD
        || source == sdk::message_category::APP_BUTTON_GROUP
    {
        return Ok(source.to_string());
    }
    let category = source.to_string();
    let suffix = if category.is_text() {
        "TEXT"
    } else if category.is_post() {
        "POST"
    } else if category.is_contact() {
        "CONTACT"
    } else if category.is_location() {
        "LOCATION"
    } else if category.is_sticker() {
        "STICKER"
    } else if category.is_live() {
        "LIVE"
    } else if category.is_transcript() {
        "TRANSCRIPT"
    } else {
        return Err(anyhow!("transcript category cannot be forwarded: {source}"));
    };
    Ok(format!("{target_prefix}_{suffix}"))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;

    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    use super::validate_sticker_image;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = RgbaImage::from_fn(width, height, |x, y| {
            Rgba([
                (x.wrapping_mul(31) ^ y.wrapping_mul(17)) as u8,
                (x.wrapping_mul(11) ^ y.wrapping_mul(47)) as u8,
                x.wrapping_add(y) as u8,
                255,
            ])
        });
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(image)
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn accepts_supported_sticker_image() {
        validate_sticker_image(Path::new("sticker.png"), &png(128, 128)).unwrap();
    }

    #[test]
    fn rejects_small_sticker_dimensions() {
        let error = validate_sticker_image(Path::new("sticker.png"), &png(127, 128)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "sticker dimensions must be between 128 and 1024 pixels"
        );
    }

    #[test]
    fn rejects_mismatched_sticker_extension() {
        let error = validate_sticker_image(Path::new("sticker.gif"), &png(128, 128)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "sticker image format does not match its file extension"
        );
    }
}
