use std::collections::HashMap;
use std::error::Error;
use std::io::{Cursor, ErrorKind};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{error, warn};
use tokio::sync::{mpsc, oneshot, watch, RwLock};
use tokio_util::sync::CancellationToken;

use sdk::api::account_api::AccountUpdateRequest;
use sdk::message_category::MessageCategory as _;
use sdk::{Account, Client, Credential, KeyStore, MessageStatus};

use crate::core::attachment::AttachmentService;
use crate::core::constants::SCP;
use crate::core::conversation_change::ConversationChangeNotifier;
use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::device_transfer::{DeviceTransferControlEvent, DeviceTransferService};
use crate::core::message::blaze::Blaze;
use crate::core::message::decrypt::{AttachmentTransferRequest, ServiceDecryptMessage};
use crate::core::message::sender::MessageSender;
use crate::core::model::auth::AuthService;
use crate::core::model::signal::SignalService;
use crate::core::model::{AppService, AttachmentExtra, ConversationService};
use crate::core::user_agent::generate_user_agent;
use crate::db::app::{Auth, SettingDao};
use crate::db::mixin::message::{MediaStatus, Message};
use crate::db::mixin::sticker::{Sticker, StickerAlbum};
use crate::db::path::account_data_directory;
use crate::db::{MixinDatabase, SignalDatabase};

const MIXIN_DATABASE_OPEN_ERROR_PREFIX: &str = "mixin_database_open_error";

mod attachment;
mod conversation;
mod conversion;
pub mod desktop;
pub mod logging;
pub mod login;
pub mod mcp;
mod message;
pub mod model;
mod sticker;
mod user;

pub use attachment::AttachmentAccess;
pub use conversation::ConversationAccess;
pub use message::MessageAccess;
pub use sticker::StickerAccess;
pub use user::UserAccess;

#[derive(Debug, thiserror::Error)]
#[error("session unauthorized")]
pub(crate) struct SessionUnauthorized;

type AccountStartupResult = std::result::Result<
    (
        Arc<MixinDatabase>,
        Arc<SignalDatabase>,
        Arc<AppService>,
        Arc<Blaze>,
        Arc<DeviceTransferService>,
        String,
    ),
    String,
>;

const MIN_STICKER_FILE_SIZE: usize = 1024;
const MAX_STICKER_FILE_SIZE: usize = 1024 * 1024;
const MIN_STICKER_DIMENSION: u32 = 128;
const MAX_STICKER_DIMENSION: u32 = 1024;
const MAX_AUDIO_FILE_SIZE: u64 = 64 * 1024 * 1024;
const MAX_AUDIO_DURATION_MILLIS: i64 = 60_000;
const MAX_AUDIO_WAVEFORM_SAMPLES: usize = 1024;

pub struct AccountRuntime {
    state: Arc<AccountState>,
    shutdown: watch::Sender<bool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

#[doc(hidden)]
pub struct AccountState {
    account_id: String,
    profile: watch::Sender<Account>,
    auth_service: Arc<AuthService>,
    client: Arc<Client>,
    database: Arc<MixinDatabase>,
    signal_database: Arc<SignalDatabase>,
    app_service: Arc<AppService>,
    conversation_changes: ConversationChangeNotifier,
    shutdown: watch::Receiver<bool>,
    notification_changes: watch::Sender<u64>,
    blaze: Arc<Blaze>,
    device_transfer: Arc<DeviceTransferService>,
    account_health: watch::Sender<String>,
    active: AtomicBool,
    mutation_gate: RwLock<()>,
    attachment_downloads: Mutex<HashMap<String, CancellationToken>>,
    attachment_progresses: Mutex<HashMap<String, f64>>,
}

impl Deref for AccountRuntime {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl AccountState {
    fn notify_conversation_changed(&self, conversation_id: &str) {
        self.conversation_changes.notify(conversation_id);
    }

    fn notify_all_conversations_changed(&self) {
        self.conversation_changes.notify_all();
    }

    fn notify_messages_changed(&self) {
        self.conversation_changes.notify_messages();
    }

    fn ensure_active(&self) -> Result<()> {
        if self.active.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(anyhow!("account runtime is shutting down"))
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

    fn set_attachment_progress(&self, message_id: &str, completed: u64, total: u64) {
        let value = if total == 0 {
            0.0
        } else {
            completed as f64 / total as f64
        }
        .clamp(0.0, 1.0);
        self.attachment_progresses
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(message_id.to_string(), value);
    }

    fn remove_attachment_progress(&self, message_id: &str) {
        self.attachment_progresses
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(message_id);
    }
}

pub struct StickerDetail {
    pub sticker: Sticker,
    pub album: Option<StickerAlbum>,
    pub album_stickers: Vec<Sticker>,
    pub is_personal: bool,
}

impl AccountRuntime {
    pub fn is_running(&self) -> bool {
        self.active.load(Ordering::Acquire)
            && self
                .thread
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
                .is_some_and(|thread| !thread.is_finished())
    }

    pub fn attachment_progress(&self, message_id: &str) -> f64 {
        self.attachment_progresses
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(message_id)
            .copied()
            .unwrap_or_default()
    }

    pub async fn start(
        auth: Auth,
        auth_service: Arc<AuthService>,
        setting_dao: SettingDao,
    ) -> Result<Self> {
        let account_id = auth.account.user_id.clone();
        let account = auth.account.clone();
        let (profile, _) = watch::channel(account);
        let client = Arc::new(Client::new_with_user_agent(
            credential(&auth),
            Some(generate_user_agent()),
        ));
        let initial_account_health = startup_account_health(client.account_api.get_me().await)?;
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let conversation_changes = ConversationChangeNotifier::new();
        let (notification_changes, _) = watch::channel(0);
        let (account_health_updates, _) = watch::channel("ready".to_string());
        let (attachment_transfer_sender, attachment_transfer_requests) = mpsc::unbounded_channel();
        let account_conversation_changes = conversation_changes.clone();
        let account_notification_changes = notification_changes.clone();
        let (ready_sender, ready_receiver) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name(format!("mixin-account-{account_id}"))
            .spawn({
                let client = client.clone();
                let account_health_updates = account_health_updates.clone();
                move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    match runtime {
                        Ok(runtime) => runtime.block_on(run_account(AccountRunContext {
                            auth,
                            client,
                            shutdown_receiver,
                            conversation_changes: account_conversation_changes,
                            notification_changes: account_notification_changes,
                            account_health_updates,
                            initial_account_health,
                            attachment_transfer_requests: attachment_transfer_sender,
                            ready_sender,
                        })),
                        Err(error) => {
                            let _ = ready_sender.send(Err(error.to_string()));
                        }
                    }
                }
            })?;
        let (
            database,
            signal_database,
            app_service,
            blaze,
            device_transfer,
            initial_account_health,
        ) = ready_receiver
            .await
            .map_err(|_| anyhow!("account runtime stopped during startup"))?
            .map_err(|error| anyhow!(error))?;

        account_health_updates.send_replace(initial_account_health);
        let state = Arc::new(AccountState {
            account_id,
            profile,
            auth_service,
            client,
            database,
            signal_database,
            app_service,
            conversation_changes,
            shutdown: shutdown.subscribe(),
            notification_changes,
            blaze,
            device_transfer,
            account_health: account_health_updates,
            active: AtomicBool::new(true),
            mutation_gate: RwLock::new(()),
            attachment_downloads: Mutex::new(HashMap::new()),
            attachment_progresses: Mutex::new(HashMap::new()),
        });
        tokio::spawn(run_attachment_transfer_requests(
            state.clone(),
            attachment_transfer_requests,
            setting_dao,
        ));
        Ok(Self {
            state,
            shutdown,
            thread: Mutex::new(Some(thread)),
        })
    }

    pub fn conversation_access(&self) -> ConversationAccess {
        ConversationAccess::new(self.state.clone())
    }

    pub fn message_access(&self) -> MessageAccess {
        MessageAccess::new(self.state.clone())
    }

    pub fn attachment_access(&self) -> AttachmentAccess {
        AttachmentAccess::new(self.state.clone())
    }

    pub fn sticker_access(&self) -> StickerAccess {
        StickerAccess::new(self.state.clone())
    }

    pub fn user_access(&self) -> UserAccess {
        UserAccess::new(self.state.clone())
    }

    pub fn device_transfer(&self) -> Arc<DeviceTransferService> {
        self.device_transfer.clone()
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn account(&self) -> Account {
        self.profile.borrow().clone()
    }

    pub fn subscribe_profile_changes(&self) -> watch::Receiver<Account> {
        self.profile.subscribe()
    }

    pub fn media_directory(&self) -> Result<PathBuf> {
        let directory = account_data_directory(&self.account().identity_number)?.join("Media");
        std::fs::create_dir_all(&directory)?;
        Ok(directory)
    }

    pub fn subscribe_conversation_changes(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::core::conversation_change::ConversationChange>
    {
        self.conversation_changes.subscribe_events()
    }

    pub fn subscribe_message_changes(&self) -> watch::Receiver<u64> {
        self.conversation_changes.subscribe_revision()
    }

    pub fn subscribe_notification_changes(&self) -> watch::Receiver<u64> {
        self.notification_changes.subscribe()
    }

    pub async fn notification_event_batch(
        &self,
        after_created_at_micros: i64,
        after_row_id: i64,
        limit: i64,
    ) -> Result<model::NotificationEventBatch> {
        let limit = limit.clamp(1, 200);
        let messages = self
            .database
            .message_dao
            .notification_items_after(&self.account_id, after_row_id, limit)
            .await?;
        let (next_created_at_micros, next_row_id) = messages
            .last()
            .map(|message| {
                (
                    message.created_at.and_utc().timestamp_micros(),
                    message.row_id,
                )
            })
            .unwrap_or((after_created_at_micros, after_row_id));
        let has_more = messages.len() == limit as usize;
        let identity_number = self.account().identity_number;
        let mut events = messages
            .into_iter()
            .filter_map(|message| {
                model::NotificationEvent::from_message(message, &self.account_id, &identity_number)
            })
            .collect::<Vec<_>>();
        let text_event_indices = events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| event.category.contains("TEXT").then_some(index))
            .collect::<Vec<_>>();
        let text_contents = text_event_indices
            .iter()
            .map(|index| events[*index].content.clone())
            .collect::<Vec<_>>();
        for (index, content) in text_event_indices.into_iter().zip(
            self.database
                .user_dao
                .replace_mentions(&text_contents)
                .await?,
        ) {
            events[index].content = content;
        }
        Ok(model::NotificationEventBatch {
            events,
            next_created_at_micros,
            next_row_id,
            has_more,
        })
    }

    pub async fn latest_notification_row_id(&self) -> Result<i64> {
        Ok(self.database.message_dao.latest_row_id().await?)
    }

    pub fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }

    pub fn subscribe_connection_status(&self) -> watch::Receiver<bool> {
        self.blaze.subscribe_connection_status()
    }

    pub fn retry_connection(&self) {
        self.blaze.retry_connection();
    }

    pub fn subscribe_account_health(&self) -> watch::Receiver<String> {
        self.account_health.subscribe()
    }

    pub async fn refresh_account_health(&self) -> Result<()> {
        let health = account_health(self.client.account_api.get_me().await)?;
        self.account_health.send_replace(health);
        self.blaze.retry_connection();
        Ok(())
    }

    pub async fn snapshot_by_trace(&self, trace_id: String) -> Result<model::SnapshotDetailItem> {
        let trace_id = trace_id.trim();
        if trace_id.is_empty() {
            return Err(anyhow!("snapshot trace id is empty"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let account = self.account();

        self.refresh_fiats().await;
        if let Ok(snapshot) = self
            .client
            .snapshot_api
            .get_snapshot_by_trace_id(trace_id)
            .await
        {
            self.database.snapshot_dao.insert(&snapshot).await?;
        }
        let mut detail = self
            .database
            .snapshot_dao
            .find_by_trace_id(trace_id, &account.fiat_currency)
            .await?
            .ok_or_else(|| anyhow!("snapshot not found"))?;

        if let Ok(asset) = self
            .client
            .asset_api
            .get_asset_by_id(&detail.asset_id)
            .await
        {
            let chain = self.client.asset_api.get_chain(&asset.chain_id).await?;
            self.database.asset_dao.insert_chain(&chain).await?;
            self.database.asset_dao.insert_asset(&asset).await?;
            detail = self
                .database
                .snapshot_dao
                .find_by_id(&detail.snapshot_id, &account.fiat_currency)
                .await?
                .ok_or_else(|| anyhow!("snapshot not found"))?;
        }

        let ticker_price_usd = self
            .ticker_price_usd(&detail.asset_id, detail.created_at)
            .await;

        Ok(model::SnapshotDetailItem::from_detail(
            detail,
            account.full_name.unwrap_or_default(),
            ticker_price_usd,
        ))
    }

    pub async fn snapshot_by_id(&self, snapshot_id: String) -> Result<model::SnapshotDetailItem> {
        let snapshot_id = snapshot_id.trim();
        if snapshot_id.is_empty() {
            return Err(anyhow!("snapshot id is empty"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let account = self.account();
        self.refresh_fiats().await;

        if let Ok(snapshot) = self
            .client
            .snapshot_api
            .get_snapshot_by_id(snapshot_id)
            .await
        {
            self.database.snapshot_dao.insert(&snapshot).await?;
        }
        let mut detail = self
            .database
            .snapshot_dao
            .find_by_id(snapshot_id, &account.fiat_currency)
            .await?
            .ok_or_else(|| anyhow!("snapshot not found"))?;
        if let Ok(asset) = self
            .client
            .asset_api
            .get_asset_by_id(&detail.asset_id)
            .await
        {
            let chain = self.client.asset_api.get_chain(&asset.chain_id).await?;
            self.database.asset_dao.insert_chain(&chain).await?;
            self.database.asset_dao.insert_asset(&asset).await?;
            detail = self
                .database
                .snapshot_dao
                .find_by_id(snapshot_id, &account.fiat_currency)
                .await?
                .ok_or_else(|| anyhow!("snapshot not found"))?;
        }
        let ticker_price_usd = self
            .ticker_price_usd(&detail.asset_id, detail.created_at)
            .await;
        Ok(model::SnapshotDetailItem::from_detail(
            detail,
            account.full_name.unwrap_or_default(),
            ticker_price_usd,
        ))
    }

    pub async fn safe_snapshot_by_id(
        &self,
        snapshot_id: String,
    ) -> Result<model::SnapshotDetailItem> {
        let snapshot_id = snapshot_id.trim();
        if snapshot_id.is_empty() {
            return Err(anyhow!("safe snapshot id is empty"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let account = self.account();
        self.refresh_fiats().await;

        if let Ok(snapshot) = self.client.token_api.get_snapshot_by_id(snapshot_id).await {
            self.database.safe_snapshot_dao.insert(&snapshot).await?;
        }
        let mut detail = self
            .database
            .safe_snapshot_dao
            .find_by_id(snapshot_id, &account.fiat_currency)
            .await?
            .ok_or_else(|| anyhow!("safe snapshot not found"))?;
        if let Ok(token) = self
            .client
            .token_api
            .get_asset_by_id(&detail.asset_id)
            .await
        {
            let chain = self.client.asset_api.get_chain(&token.chain_id).await?;
            self.database.asset_dao.insert_chain(&chain).await?;
            self.database.asset_dao.insert_token(&token).await?;
            detail = self
                .database
                .safe_snapshot_dao
                .find_by_id(snapshot_id, &account.fiat_currency)
                .await?
                .ok_or_else(|| anyhow!("safe snapshot not found"))?;
        }
        let ticker_price_usd = self
            .ticker_price_usd(&detail.asset_id, detail.created_at)
            .await;
        Ok(model::SnapshotDetailItem::from_safe_detail(
            detail,
            account.full_name.unwrap_or_default(),
            ticker_price_usd,
        ))
    }

    async fn refresh_fiats(&self) {
        match self.client.account_api.get_fiats().await {
            Ok(fiats) => {
                if let Err(error) = self.database.fiat_dao.insert_all(&fiats).await {
                    warn!("failed to persist fiat rates: {error}");
                }
            }
            Err(error) => warn!("failed to refresh fiat rates: {error}"),
        }
    }

    async fn ticker_price_usd(
        &self,
        asset_id: &str,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Option<String> {
        let offset = created_at.to_rfc3339();
        self.client
            .snapshot_api
            .get_ticker(asset_id, Some(&offset))
            .await
            .ok()
            .map(|ticker| ticker.price_usd)
    }

    pub async fn update_account_profile(
        &self,
        full_name: String,
        biography: String,
    ) -> Result<Account> {
        let full_name = full_name.trim();
        let biography = biography.trim();
        if full_name.is_empty() {
            return Err(anyhow!("account name is required"));
        }
        if full_name.chars().count() > 40 {
            return Err(anyhow!("account name exceeds 40 characters"));
        }
        if biography.chars().count() > 140 {
            return Err(anyhow!("account biography exceeds 140 characters"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let account = self
            .client
            .account_api
            .update(&AccountUpdateRequest {
                full_name: Some(full_name),
                avatar_base64: None,
                biography: Some(biography),
            })
            .await?;
        self.persist_account_profile(account).await
    }

    pub async fn update_account_avatar(&self, avatar_base64: String) -> Result<Account> {
        let avatar_base64 = avatar_base64.trim();
        if avatar_base64.is_empty() {
            return Err(anyhow!("account avatar is required"));
        }
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let account = self
            .client
            .account_api
            .update(&AccountUpdateRequest {
                full_name: None,
                avatar_base64: Some(avatar_base64),
                biography: None,
            })
            .await?;
        self.persist_account_profile(account).await
    }

    pub async fn refresh_account_profile(&self) -> Result<Account> {
        let _mutation = self.mutation_gate.read().await;
        self.ensure_active()?;
        let account = self.client.account_api.get_me().await?;
        self.persist_account_profile(account).await
    }

    async fn persist_account_profile(&self, account: Account) -> Result<Account> {
        if account.user_id != self.account_id {
            return Err(anyhow!("profile account does not match the active account"));
        }
        let mut auth = self
            .auth_service
            .get_auth()
            .ok_or_else(|| anyhow!("account authorization not found"))?;
        if auth.user_id != self.account_id {
            return Err(anyhow!(
                "saved authorization does not match the active account"
            ));
        }
        auth.account = account.clone();
        self.auth_service.save_auth(&auth).await?;
        self.profile.send_replace(account.clone());
        Ok(account)
    }

    pub async fn storage_usage(&self) -> Result<Vec<model::ConversationStorageUsage>> {
        self.ensure_active()?;
        let mut conversations = Vec::new();
        let mut offset = 0;
        loop {
            let page = self
                .conversation_access()
                .conversations("chats".to_string(), None, String::new(), false, 200, offset)
                .await?;
            let page_len = page.len();
            conversations.extend(page);
            if page_len < 200 {
                break;
            }
            offset += page_len as i64;
        }
        let media = account_data_directory(&self.account().identity_number)?.join("Media");
        tokio::task::spawn_blocking(move || {
            let mut usage = conversations
                .into_iter()
                .map(|conversation| {
                    let size_bytes = ["Images", "Videos", "Audios", "Files"]
                        .into_iter()
                        .map(|category| {
                            directory_size(
                                &media.join(category).join(&conversation.conversation_id),
                            )
                        })
                        .sum::<u64>();
                    model::ConversationStorageUsage {
                        conversation,
                        size_bytes: i64::try_from(size_bytes).unwrap_or(i64::MAX),
                    }
                })
                .collect::<Vec<_>>();
            usage.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
            usage
        })
        .await
        .map_err(|error| anyhow!("storage usage task failed: {error}"))
    }

    pub async fn conversation_storage_usage(
        &self,
        conversation_id: String,
    ) -> Result<Vec<model::StorageCategoryUsage>> {
        validate_storage_component("conversation id", &conversation_id)?;
        let media = account_data_directory(&self.account().identity_number)?.join("Media");
        tokio::task::spawn_blocking(move || {
            [
                ("photos", "Images"),
                ("videos", "Videos"),
                ("audio", "Audios"),
                ("files", "Files"),
            ]
            .into_iter()
            .map(|(category, directory)| model::StorageCategoryUsage {
                category: category.to_string(),
                size_bytes: i64::try_from(directory_size(
                    &media.join(directory).join(&conversation_id),
                ))
                .unwrap_or(i64::MAX),
            })
            .collect()
        })
        .await
        .map_err(|error| anyhow!("conversation storage usage task failed: {error}"))
    }

    pub async fn clear_conversation_storage(
        &self,
        conversation_id: String,
        categories: Vec<String>,
    ) -> Result<()> {
        validate_storage_component("conversation id", &conversation_id)?;
        let directories = categories
            .into_iter()
            .map(|category| match category.as_str() {
                "photos" => Ok("Images"),
                "videos" => Ok("Videos"),
                "audio" => Ok("Audios"),
                "files" => Ok("Files"),
                _ => Err(anyhow!("invalid storage category: {category}")),
            })
            .collect::<Result<Vec<_>>>()?;
        let media = account_data_directory(&self.account().identity_number)?.join("Media");
        tokio::task::spawn_blocking(move || {
            for directory in directories {
                clear_directory_contents(&media.join(directory).join(&conversation_id))?;
            }
            Ok(())
        })
        .await
        .map_err(|error| anyhow!("clear storage task failed: {error}"))?
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
        let session_id = self.account().session_id;
        if let Err(error) = self.client.account_api.logout(&session_id).await {
            warn!("failed to revoke account session during sign out: {error}");
        }
    }

    pub fn begin_sign_out(&self) {
        self.active.store(false, Ordering::Release);
        self.cancel_attachment_downloads();
    }

    pub async fn sign_out(&self) -> Result<()> {
        let data_directory = account_data_directory(&self.account().identity_number)?;
        self.begin_sign_out();
        let clear_auth_error = self.auth_service.clear_auth(&self.account_id).await.err();
        let _mutation = self.mutation_gate.write().await;
        let session_id = self.account().session_id;
        self.shutdown_inner().await;
        self.revoke_session().await;
        if let Err(error) = self.signal_database.clear().await {
            warn!("failed to clear signal state during sign out: {error}");
        }
        if let Err(error) = self
            .database
            .participant_session_dao
            .clear_for_sign_out(&session_id)
            .await
        {
            warn!("failed to clear participant sessions during sign out: {error}");
        }
        self.signal_database.close().await;
        self.database.close().await;
        match tokio::fs::remove_dir_all(&data_directory).await {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => warn!(
                "failed to clear account data directory {} after sign out: {error}",
                data_directory.display()
            ),
        }
        if let Some(error) = clear_auth_error {
            warn!("failed to clear local auth after sign out: {error}");
        }
        Ok(())
    }
}

async fn run_attachment_transfer_requests(
    state: Arc<AccountState>,
    mut requests: mpsc::UnboundedReceiver<AttachmentTransferRequest>,
    setting_dao: SettingDao,
) {
    let mut shutdown = state.shutdown.clone();
    loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(request) = request else {
                    return;
                };
                if request.cancel {
                    let attachment = AttachmentAccess::new(state.clone());
                    let result = match request.transcript_id {
                        Some(transcript_id) => attachment
                            .cancel_transcript_attachment(transcript_id, request.message_id.clone())
                            .await,
                        None => attachment.cancel_attachment(request.message_id.clone()).await,
                    };
                    if let Err(error) = result {
                        error!(
                            "automatic attachment cancellation failed: message_id={}, error={error:?}",
                            request.message_id,
                        );
                    }
                    continue;
                }
                let should_download = match setting_dao
                    .should_auto_download(&request.category)
                    .await
                {
                    Ok(should_download) => should_download,
                    Err(error) => {
                        error!(
                            "failed to load attachment auto-download settings: message_id={}, category={}, error={error:?}",
                            request.message_id,
                            request.category,
                        );
                        continue;
                    }
                };
                if !should_download {
                    continue;
                }
                let attachment = AttachmentAccess::new(state.clone());
                tokio::spawn(async move {
                    let result = match request.transcript_id.as_ref() {
                        Some(transcript_id) => attachment
                            .download_transcript_attachment(
                                transcript_id.clone(),
                                request.message_id.clone(),
                            )
                            .await,
                        None => attachment.download_attachment(request.message_id.clone()).await,
                    };
                    if let Err(error) = result {
                        error!(
                            "automatic attachment download failed: message_id={}, transcript_id={:?}, category={}, error={error:?}",
                            request.message_id,
                            request.transcript_id,
                            request.category,
                        );
                    }
                });
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return;
                }
            }
        }
    }
}

fn validate_storage_component(label: &str, value: &str) -> Result<()> {
    let mut components = Path::new(value).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(anyhow!("invalid {label}"));
    }
    Ok(())
}

fn directory_size(path: &Path) -> u64 {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.file_type().is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        return metadata.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(std::result::Result::ok)
        .map(|entry| directory_size(&entry.path()))
        .sum()
}

fn clear_directory_contents(path: &PathBuf) -> Result<()> {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(anyhow!("invalid storage directory: {}", path.display()));
    }
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let metadata = std::fs::symlink_metadata(&child)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            std::fs::remove_dir_all(child)?;
        } else {
            std::fs::remove_file(child)?;
        }
    }
    Ok(())
}

impl Drop for AccountRuntime {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        self.cancel_attachment_downloads();
        let _ = self.shutdown.send(true);
    }
}

struct AccountRunContext {
    auth: Auth,
    client: Arc<Client>,
    shutdown_receiver: watch::Receiver<bool>,
    conversation_changes: ConversationChangeNotifier,
    notification_changes: watch::Sender<u64>,
    account_health_updates: watch::Sender<String>,
    initial_account_health: String,
    attachment_transfer_requests: mpsc::UnboundedSender<AttachmentTransferRequest>,
    ready_sender: oneshot::Sender<AccountStartupResult>,
}

async fn run_account(context: AccountRunContext) {
    let AccountRunContext {
        auth,
        client,
        mut shutdown_receiver,
        conversation_changes,
        notification_changes,
        account_health_updates,
        initial_account_health,
        attachment_transfer_requests,
        ready_sender,
    } = context;
    let result = prepare_account(
        &auth,
        client.clone(),
        conversation_changes,
        notification_changes,
        initial_account_health,
        attachment_transfer_requests,
    )
    .await;
    let (
        database,
        signal_database,
        blaze,
        decrypt_message,
        sender,
        app_service,
        device_transfer,
        device_transfer_controls,
        account_health,
    ) = match result {
        Ok(services) => services,
        Err(error) => {
            let _ = ready_sender.send(Err(error.to_string()));
            return;
        }
    };
    if ready_sender
        .send(Ok((
            database,
            signal_database,
            app_service.clone(),
            blaze.clone(),
            device_transfer.clone(),
            account_health,
        )))
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
        _ = device_transfer.run(device_transfer_controls) => {
            warn!("device transfer service stopped");
        }
        result = app_service.job.start() => {
            if let Err(error) = result {
                error!("job service stopped: {error:?}");
            }
        }
        _ = forward_account_health(client, account_health_updates) => {
            warn!("account health monitor stopped");
        }
        _ = shutdown_receiver.changed() => {}
    }
}

async fn forward_account_health(client: Arc<Client>, health: watch::Sender<String>) {
    let mut errors = client.subscribe_server_error_codes();
    loop {
        if let Some(code) = *errors.borrow_and_update() {
            let value = match code {
                sdk::err::error_code::TIME_INACCURATE => "time_inaccurate",
                sdk::err::error_code::OLD_VERSION => "update_required",
                _ => "ready",
            };
            if value != "ready" {
                health.send_replace(value.to_string());
            }
        }
        if errors.changed().await.is_err() {
            return;
        }
    }
}

type AccountServices = (
    Arc<MixinDatabase>,
    Arc<SignalDatabase>,
    Arc<Blaze>,
    Arc<ServiceDecryptMessage>,
    Arc<MessageSender>,
    Arc<AppService>,
    Arc<DeviceTransferService>,
    tokio::sync::broadcast::Receiver<DeviceTransferControlEvent>,
    String,
);

async fn prepare_account(
    auth: &Auth,
    client: Arc<Client>,
    conversation_changes: ConversationChangeNotifier,
    notification_changes: watch::Sender<u64>,
    account_health: String,
    attachment_transfer_requests: mpsc::UnboundedSender<AttachmentTransferRequest>,
) -> Result<AccountServices> {
    let account = &auth.account;
    let account_id = account.user_id.clone();
    let credential = credential(auth);

    let database = Arc::new(
        MixinDatabase::new(account.identity_number.clone())
            .await
            .map_err(|error| database_open_error(error.as_ref()))?,
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
    let signal_service = SignalService::new(
        signal_protocol.clone(),
        signal_database.clone(),
        client.clone(),
    );
    let sender = Arc::new(MessageSender::new(
        blaze.clone(),
        conversation,
        database.clone(),
        account_id.clone(),
        account.session_id.clone(),
        signal_protocol.clone(),
        signal_service,
    ));
    let account_data_dir = account_data_directory(&account.identity_number)?;
    let attachment = Arc::new(AttachmentService::new(
        client.clone(),
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(150))
            .build()?,
        account_data_dir.clone(),
    ));
    let app_service = Arc::new(AppService::new(
        database.clone(),
        client.clone(),
        auth,
        sender.clone(),
        attachment,
        Some(conversation_changes.clone()),
    ));
    let (device_transfer_control_sender, device_transfer_controls) =
        tokio::sync::broadcast::channel(16);
    let device_transfer = DeviceTransferService::new(
        database.clone(),
        sender.clone(),
        account_id,
        account.session_id.clone(),
        auth.primary_session_id.clone(),
        account_data_dir,
        conversation_changes.clone(),
    );
    let decrypt_message = Arc::new(
        ServiceDecryptMessage::new(
            database.clone(),
            app_service.clone(),
            signal_protocol,
            sender.clone(),
            blaze.pending_message_statuses(),
            auth,
        )
        .with_conversation_changes(conversation_changes)
        .with_notification_changes(notification_changes)
        .with_device_transfer_controls(device_transfer_control_sender)
        .with_attachment_transfer_requests(attachment_transfer_requests),
    );
    Ok((
        database,
        signal_database,
        blaze,
        decrypt_message,
        sender,
        app_service,
        device_transfer,
        device_transfer_controls,
        account_health,
    ))
}

fn database_open_error(error: &(dyn Error + 'static)) -> anyhow::Error {
    let Some(code) = sqlite_result_code(error) else {
        return anyhow!(error.to_string());
    };
    anyhow!("{MIXIN_DATABASE_OPEN_ERROR_PREFIX}:{code}:{error}")
}

fn sqlite_result_code(mut error: &(dyn Error + 'static)) -> Option<i32> {
    loop {
        if let Some(sqlx_error) = error.downcast_ref::<sqlx::Error>() {
            return sqlx_error
                .as_database_error()
                .and_then(|database_error| database_error.code())
                .and_then(|code| code.parse().ok());
        }
        error = error.source()?;
    }
}

fn account_health<T>(result: std::result::Result<T, sdk::ApiError>) -> Result<String> {
    match result {
        Ok(_) => Ok("ready".to_string()),
        Err(sdk::ApiError::Server(error))
            if error.code == sdk::err::error_code::TIME_INACCURATE =>
        {
            Ok("time_inaccurate".to_string())
        }
        Err(sdk::ApiError::Server(error)) if error.code == sdk::err::error_code::OLD_VERSION => {
            Ok("update_required".to_string())
        }
        Err(error) => Err(anyhow!(error.to_string())),
    }
}

fn startup_account_health<T>(result: std::result::Result<T, sdk::ApiError>) -> Result<String> {
    match result {
        Ok(_) => Ok("ready".to_string()),
        Err(sdk::ApiError::Server(error)) if error.code == sdk::err::error_code::AUTHENTICATION => {
            Err(SessionUnauthorized.into())
        }
        Err(sdk::ApiError::Server(error))
            if error.code == sdk::err::error_code::TIME_INACCURATE =>
        {
            Ok("time_inaccurate".to_string())
        }
        Err(sdk::ApiError::Server(error)) if error.code == sdk::err::error_code::OLD_VERSION => {
            Ok("update_required".to_string())
        }
        Err(error) => {
            warn!("account health probe failed during cached startup: {error}");
            Ok("ready".to_string())
        }
    }
}

pub fn credential(auth: &Auth) -> Credential {
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

    use super::{startup_account_health, validate_sticker_image, SessionUnauthorized};

    fn server_error(code: i64) -> sdk::ApiError {
        sdk::ApiError::Server(sdk::Error {
            status: 400,
            code,
            description: "test".to_string(),
        })
    }

    #[test]
    fn cached_startup_remains_ready_when_account_probe_has_transport_failure() {
        let error = reqwest::Client::new().get("://").build().unwrap_err();

        let health = startup_account_health::<()>(Err(sdk::ApiError::Request(error))).unwrap();

        assert_eq!(health, "ready");
    }

    #[test]
    fn cached_startup_requires_login_when_account_probe_confirms_unauthorized() {
        let error =
            startup_account_health::<()>(Err(server_error(sdk::err::error_code::AUTHENTICATION)))
                .unwrap_err();

        assert!(error.downcast_ref::<SessionUnauthorized>().is_some());
    }

    #[test]
    fn cached_startup_preserves_actionable_account_health_errors() {
        for (code, expected) in [
            (sdk::err::error_code::TIME_INACCURATE, "time_inaccurate"),
            (sdk::err::error_code::OLD_VERSION, "update_required"),
        ] {
            let health = startup_account_health::<()>(Err(server_error(code))).unwrap();
            assert_eq!(health, expected);
        }
    }

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
