use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{error, warn};
use tokio::sync::{oneshot, watch, RwLock};
use tokio_util::sync::CancellationToken;

use sdk::message_category::MessageCategory as _;
use sdk::{Account, Client, Credential, KeyStore, MessageStatus};

use crate::core::attachment::AttachmentService;
use crate::core::constants::SCP;
use crate::core::crypto::signal_protocol::SignalProtocol;
use crate::core::message::blaze::Blaze;
use crate::core::message::decrypt::ServiceDecryptMessage;
use crate::core::message::sender::MessageSender;
use crate::core::model::signal::SignalService;
use crate::core::model::{AppService, AttachmentExtra, ConversationService};
use crate::db::app::Auth;
use crate::db::mixin::message::{MediaStatus, Message};
use crate::db::mixin::sticker::{Sticker, StickerAlbum};
use crate::db::path::account_data_directory;
use crate::db::{MixinDatabase, SignalDatabase};

mod attachment;
mod conversation;
mod conversion;
mod message;
pub mod model;
mod sticker;
mod user;

pub use attachment::AttachmentAccess;
pub use conversation::ConversationAccess;
pub use message::MessageAccess;
pub use sticker::StickerAccess;
pub use user::UserAccess;

type AccountStartupResult =
    std::result::Result<(Arc<MixinDatabase>, Arc<SignalDatabase>, Arc<AppService>), String>;

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
    account: Account,
    client: Arc<Client>,
    database: Arc<MixinDatabase>,
    signal_database: Arc<SignalDatabase>,
    app_service: Arc<AppService>,
    conversation_changes: watch::Sender<u64>,
    active: AtomicBool,
    mutation_gate: RwLock<()>,
    attachment_downloads: Mutex<HashMap<String, CancellationToken>>,
}

impl Deref for AccountRuntime {
    type Target = AccountState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl AccountState {
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

pub struct StickerDetail {
    pub sticker: Sticker,
    pub album: Option<StickerAlbum>,
    pub album_stickers: Vec<Sticker>,
    pub is_personal: bool,
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
            state: Arc::new(AccountState {
                account_id,
                account,
                client,
                database,
                signal_database,
                app_service,
                conversation_changes,
                active: AtomicBool::new(true),
                mutation_gate: RwLock::new(()),
                attachment_downloads: Mutex::new(HashMap::new()),
            }),
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

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    pub fn account(&self) -> &Account {
        &self.account
    }

    pub fn subscribe_conversation_changes(&self) -> watch::Receiver<u64> {
        self.conversation_changes.subscribe()
    }

    pub fn subscribe_shutdown(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
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
