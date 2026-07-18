use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex, RwLock};

use anyhow::Result;
use chrono::Local;
use log::{warn, LevelFilter, Log, Metadata, Record, SetLoggerError};
use mixin_desktop_core::core::model::auth::{AuthService, AuthorizationSession};
use mixin_desktop_core::db::app::{AppDatabase, PropertyDao};
use mixin_desktop_core::db::path::{account_data_directory, log_directory};
use mixin_desktop_core::db::SignalDatabase;
use mixin_desktop_core::network::{
    NetworkService, ProxyConfig, ProxySettings, ProxyType, SharedNetworkService,
};
use mixin_desktop_core::runtime::model::{
    AccountProfile, ConversationStorageUsage, SnapshotDetailItem, StorageCategoryUsage,
};
use mixin_desktop_core::runtime::{
    credential, AccountRuntime, AttachmentAccess, ConversationAccess, MessageAccess, StickerAccess,
    UserAccess,
};
use sdk::Client;
use tokio::sync::Mutex;

use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopHandle {
    auth_service: Arc<AuthService>,
    network_service: SharedNetworkService,
    property_dao: PropertyDao,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct LoginHandle {
    auth_service: Arc<AuthService>,
    session: Mutex<Option<AuthorizationSession>>,
    auth_url: String,
    property_dao: PropertyDao,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct AccountHandle {
    runtime: Arc<AccountRuntime>,
    auth_service: Arc<AuthService>,
    profile: RwLock<sdk::Account>,
}

pub struct ProxyItem {
    pub id: String,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub struct ProxySettingsItem {
    pub enabled: bool,
    pub selected_proxy_id: Option<String>,
    pub proxies: Vec<ProxyItem>,
}

pub struct HttpResponseItem {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

struct FlutterFileLogger {
    file: StdMutex<File>,
}

static RUST_LOGGER_INITIALIZED: StdMutex<bool> = StdMutex::new(false);

impl Log for FlutterFileLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.target() == "flutter" || metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} {:<5} {} - {}",
            Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            record.level(),
            record.target(),
            record.args(),
        );
        println!("{line}");
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

fn install_rust_logger(file: File) -> std::result::Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(FlutterFileLogger {
        file: StdMutex::new(file),
    }))?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}

impl From<ProxyConfig> for ProxyItem {
    fn from(proxy: ProxyConfig) -> Self {
        Self {
            id: proxy.id,
            kind: match proxy.proxy_type {
                ProxyType::Http => "http".to_string(),
                ProxyType::Socks5 => "socks5".to_string(),
            },
            host: proxy.host,
            port: proxy.port,
            username: proxy.username,
            password: proxy.password,
        }
    }
}

impl TryFrom<ProxyItem> for ProxyConfig {
    type Error = anyhow::Error;

    fn try_from(proxy: ProxyItem) -> Result<Self> {
        let proxy_type = match proxy.kind.to_lowercase().as_str() {
            "http" => ProxyType::Http,
            "socks5" => ProxyType::Socks5,
            _ => anyhow::bail!("unsupported proxy type"),
        };
        Ok(Self {
            id: proxy.id,
            proxy_type,
            host: proxy.host,
            port: proxy.port,
            username: proxy.username,
            password: proxy.password,
        })
    }
}

impl From<ProxySettings> for ProxySettingsItem {
    fn from(settings: ProxySettings) -> Self {
        Self {
            enabled: settings.enabled,
            selected_proxy_id: settings.selected_proxy_id,
            proxies: settings.proxies.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<ProxySettingsItem> for ProxySettings {
    type Error = anyhow::Error;

    fn try_from(settings: ProxySettingsItem) -> Result<Self> {
        Ok(Self {
            enabled: settings.enabled,
            selected_proxy_id: settings.selected_proxy_id,
            proxies: settings
                .proxies
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

fn init_rust_logger() -> Result<()> {
    let mut initialized = RUST_LOGGER_INITIALIZED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *initialized {
        return Ok(());
    }

    let directory = log_directory()?;
    std::fs::create_dir_all(&directory)?;
    let now = Local::now();
    let log_file_path = directory.join(format!("{}.log", now.format("%Y-%m-%d")));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;
    install_rust_logger(file)?;
    flutter_rust_bridge::setup_backtrace();
    *initialized = true;
    log::info!(target: "logger", "initialized at {}", directory.display());
    Ok(())
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() -> Result<()> {
    // Do not call `setup_default_user_utils`: it attempts to install `oslog`, while the
    // global `log` logger can only be set once. Set up only its backtrace behavior below.
    init_rust_logger()
}

#[flutter_rust_bridge::frb(sync)]
pub fn rust_log_directory() -> Result<String> {
    Ok(log_directory()?.to_string_lossy().into_owned())
}

#[flutter_rust_bridge::frb(sync)]
pub fn log_flutter(level: String, message: String) {
    match level.as_str() {
        "verbose" => log::trace!(target: "flutter", "{message}"),
        "debug" => log::debug!(target: "flutter", "{message}"),
        "info" => log::info!(target: "flutter", "{message}"),
        "warning" => log::warn!(target: "flutter", "{message}"),
        "error" | "wtf" => log::error!(target: "flutter", "{message}"),
        _ => log::info!(target: "flutter", "{message}"),
    }
}

pub async fn open_desktop() -> Result<DesktopHandle> {
    let database = Arc::new(AppDatabase::connect().await?);
    let network_service = Arc::new(NetworkService::new(database.property_dao.clone()).await?);
    let auth_service = Arc::new(AuthService::new(database.clone()));
    auth_service.initialize().await?;
    Ok(DesktopHandle {
        auth_service,
        network_service,
        property_dao: database.property_dao.clone(),
    })
}

impl DesktopHandle {
    pub async fn proxy_settings(&self) -> Result<ProxySettingsItem> {
        Ok(self.network_service.proxy_settings().await.into())
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettingsItem) -> Result<()> {
        self.network_service
            .set_proxy_settings(settings.try_into()?)
            .await
    }

    pub async fn http_request(
        &self,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        timeout_millis: Option<u64>,
        max_response_bytes: Option<u64>,
    ) -> Result<HttpResponseItem> {
        let response = self
            .network_service
            .request(
                &method,
                &url,
                headers,
                body,
                timeout_millis,
                max_response_bytes.map(|value| value as usize),
            )
            .await?;
        Ok(HttpResponseItem {
            status_code: response.status_code,
            headers: response.headers,
            body: response.body,
        })
    }

    pub async fn restore_account(&self) -> Result<Option<AccountHandle>> {
        let Some(auth) = self.auth_service.get_auth() else {
            return Ok(None);
        };
        if !device_matches(
            &self.property_dao,
            &self.auth_service,
            &auth.account.user_id,
            &auth.account.identity_number,
        )
        .await?
        {
            return Ok(None);
        }
        let signal_database = SignalDatabase::connect(auth.account.identity_number.clone())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        if signal_database
            .identity_dao
            .get_local_identity()
            .await?
            .is_none()
        {
            self.auth_service.clear_auth(&auth.account.user_id).await?;
            return Ok(None);
        }
        let profile = auth.account.clone();
        let runtime = AccountRuntime::start(auth).await?;
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
            auth_service: self.auth_service.clone(),
            profile: RwLock::new(profile),
        }))
    }

    pub async fn recreate_account_database(&self) -> Result<()> {
        let auth = self
            .auth_service
            .get_auth()
            .ok_or_else(|| anyhow::anyhow!("no saved account"))?;
        let database = account_data_directory(&auth.account.identity_number)?.join("mixin.db");
        rename_with_timestamp_if_exists(&database).await?;
        remove_if_exists(&database.with_extension("db-shm")).await?;
        remove_if_exists(&database.with_extension("db-wal")).await?;
        Ok(())
    }

    pub async fn abort_saved_login(&self) -> Result<()> {
        let Some(auth) = self.auth_service.get_auth() else {
            return Ok(());
        };
        let client = Client::new(credential(&auth));
        if let Err(error) = client.account_api.logout(&auth.account.session_id).await {
            warn!("failed to revoke session after login startup failure: {error}");
        }
        self.auth_service.clear_auth(&auth.account.user_id).await?;
        let database = match SignalDatabase::connect(auth.account.identity_number).await {
            Ok(database) => database,
            Err(error) => {
                warn!("failed to open signal state after login startup failure: {error}");
                return Ok(());
            }
        };
        if let Err(error) = database.clear().await {
            warn!("failed to clear signal state after login startup failure: {error}");
        }
        database.close().await;
        Ok(())
    }

    pub async fn begin_login(&self) -> Result<LoginHandle> {
        let session = self
            .auth_service
            .begin_authorization(desktop_platform())
            .await?;
        let auth_url = session.auth_url().to_string();
        Ok(LoginHandle {
            auth_service: self.auth_service.clone(),
            session: Mutex::new(Some(session)),
            auth_url,
            property_dao: self.property_dao.clone(),
        })
    }
}

async fn rename_with_timestamp_if_exists(path: &Path) -> Result<()> {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(anyhow::anyhow!("database path has no file name"));
    };
    let target = path.with_file_name(format!("{file_name}.{}", Local::now().to_rfc3339()));
    match tokio::fs::rename(path, target).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn remove_if_exists(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

impl LoginHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn auth_url(&self) -> String {
        self.auth_url.clone()
    }

    pub async fn poll(&self) -> Result<Option<AccountHandle>> {
        let mut session = self.session.lock().await;
        let Some(active_session) = session.as_ref() else {
            return Ok(None);
        };
        let Some(result) = self.auth_service.poll_authorization(active_session).await? else {
            return Ok(None);
        };
        let auth = self.auth_service.complete_authorization(result).await?;
        record_current_device(&self.property_dao).await?;
        let profile = auth.account.clone();
        let runtime = AccountRuntime::start(auth)
            .await
            .map_err(|error| anyhow::anyhow!("login_provisioning_error:{error}"))?;
        session.take();
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
            auth_service: self.auth_service.clone(),
            profile: RwLock::new(profile),
        }))
    }
}

impl AccountHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn account_id(&self) -> String {
        self.runtime.account_id().to_string()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn profile(&self) -> AccountProfile {
        account_profile(
            &self
                .profile
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    pub async fn update_profile(
        &self,
        full_name: String,
        biography: String,
    ) -> Result<AccountProfile> {
        let account = self
            .runtime
            .update_account_profile(full_name, biography)
            .await?;
        self.persist_profile(account).await
    }

    pub async fn refresh_profile(&self) -> Result<AccountProfile> {
        let account = self.runtime.refresh_account_profile().await?;
        self.persist_profile(account).await
    }

    async fn persist_profile(&self, account: sdk::Account) -> Result<AccountProfile> {
        let mut auth = self
            .auth_service
            .get_auth()
            .ok_or_else(|| anyhow::anyhow!("account authorization not found"))?;
        auth.account = account.clone();
        self.auth_service.save_auth(&auth).await?;
        *self
            .profile
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = account.clone();
        Ok(account_profile(&account))
    }

    pub async fn storage_usage(&self) -> Result<Vec<ConversationStorageUsage>> {
        self.runtime.storage_usage().await
    }

    pub async fn snapshot_by_trace(&self, trace_id: String) -> Result<SnapshotDetailItem> {
        self.runtime.snapshot_by_trace(trace_id).await
    }

    pub async fn snapshot_by_id(&self, snapshot_id: String) -> Result<SnapshotDetailItem> {
        self.runtime.snapshot_by_id(snapshot_id).await
    }

    pub async fn safe_snapshot_by_id(&self, snapshot_id: String) -> Result<SnapshotDetailItem> {
        self.runtime.safe_snapshot_by_id(snapshot_id).await
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn media_directory(&self) -> Result<String> {
        let directory = account_data_directory(&self.profile().identity_number)?.join("Media");
        std::fs::create_dir_all(&directory)?;
        Ok(directory.to_string_lossy().into_owned())
    }

    pub async fn conversation_storage_usage(
        &self,
        conversation_id: String,
    ) -> Result<Vec<StorageCategoryUsage>> {
        self.runtime
            .conversation_storage_usage(conversation_id)
            .await
    }

    pub async fn clear_conversation_storage(
        &self,
        conversation_id: String,
        categories: Vec<String>,
    ) -> Result<()> {
        self.runtime
            .clear_conversation_storage(conversation_id, categories)
            .await
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn conversation(&self) -> ConversationAccess {
        self.runtime.conversation_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn message(&self) -> MessageAccess {
        self.runtime.message_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn attachment(&self) -> AttachmentAccess {
        self.runtime.attachment_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn download_progress(&self, message_id: String) -> f64 {
        self.runtime.attachment_progress(&message_id)
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn sticker(&self) -> StickerAccess {
        self.runtime.sticker_access()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn user(&self) -> UserAccess {
        self.runtime.user_access()
    }

    pub async fn conversation_changes(&self, sink: StreamSink<u64>) -> Result<()> {
        forward_changes(&self.runtime, sink).await
    }

    pub async fn message_changes(&self, sink: StreamSink<u64>) -> Result<()> {
        forward_changes(&self.runtime, sink).await
    }

    pub async fn device_transfer_events(&self, sink: StreamSink<String>) -> Result<()> {
        let mut events = self.runtime.device_transfer().subscribe();
        let mut shutdown = self.runtime.subscribe_shutdown();
        loop {
            tokio::select! {
                event = events.recv() => match event {
                    Ok(event) => sink
                        .add(serde_json::to_string(&event)?)
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
                },
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn device_transfer_command(&self, command: String) -> Result<()> {
        self.runtime.device_transfer().command(&command).await
    }

    pub async fn connection_status(&self, sink: StreamSink<bool>) -> Result<()> {
        let mut status = self.runtime.subscribe_connection_status();
        let mut shutdown = self.runtime.subscribe_shutdown();
        sink.add(*status.borrow())
            .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        loop {
            tokio::select! {
                changed = status.changed() => {
                    if changed.is_err() {
                        return Ok(());
                    }
                    sink.add(*status.borrow_and_update())
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn retry_connection(&self) {
        self.runtime.retry_connection();
    }

    pub async fn account_health(&self, sink: StreamSink<String>) -> Result<()> {
        let mut health = self.runtime.subscribe_account_health();
        let mut shutdown = self.runtime.subscribe_shutdown();
        sink.add(health.borrow().clone())
            .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        loop {
            tokio::select! {
                changed = health.changed() => {
                    if changed.is_err() {
                        return Ok(());
                    }
                    sink.add(health.borrow_and_update().clone())
                        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub async fn refresh_account_health(&self) -> Result<()> {
        self.runtime.refresh_account_health().await
    }

    pub async fn shutdown(&self) {
        self.runtime.shutdown().await;
    }

    pub async fn sign_out(&self) -> Result<()> {
        let data_directory = account_data_directory(&self.profile().identity_number)?;
        self.runtime.begin_sign_out();
        let clear_auth_error = self
            .auth_service
            .clear_auth(self.runtime.account_id())
            .await
            .err();
        self.runtime.sign_out().await;
        match tokio::fs::remove_dir_all(&data_directory).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
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

const DEVICE_PROPERTY_GROUP: &str = "account";
const DEVICE_PROPERTY_KEY: &str = "device_id";

async fn device_matches(
    property_dao: &PropertyDao,
    auth_service: &AuthService,
    user_id: &str,
    identity_number: &str,
) -> Result<bool> {
    let Some(current) = current_device_id().await else {
        return Ok(true);
    };
    let saved = property_dao
        .get(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY)
        .await?;
    if saved
        .as_deref()
        .is_none_or(|saved| saved.eq_ignore_ascii_case(&current))
    {
        property_dao
            .set(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY, &current)
            .await?;
        return Ok(true);
    }

    auth_service.clear_auth(user_id).await?;
    let directory = account_data_directory(identity_number)?;
    match tokio::fs::remove_dir_all(&directory).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    property_dao
        .set(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY, &current)
        .await?;
    Ok(false)
}

async fn record_current_device(property_dao: &PropertyDao) -> Result<()> {
    let Some(current) = current_device_id().await else {
        return Ok(());
    };
    property_dao
        .set(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY, &current)
        .await?;
    Ok(())
}

async fn current_device_id() -> Option<String> {
    tokio::task::spawn_blocking(current_device_id_sync)
        .await
        .ok()
        .flatten()
}

fn current_device_id_sync() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let id = std::fs::read_to_string("/etc/machine-id").ok()?;
        let id = id.trim();
        return (id.len() == 32 && id.chars().all(|character| character.is_ascii_hexdigit()))
            .then(|| id.to_lowercase());
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok()?;
        let output = String::from_utf8(output.stdout).ok()?;
        return output.lines().find_map(|line| {
            if !line.contains("IOPlatformUUID") {
                return None;
            }
            line.split('=')
                .nth(1)
                .map(|value| value.trim().trim_matches('"').to_lowercase())
                .filter(|value| !value.is_empty())
        });
    }
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("wmic")
            .args(["csproduct", "get", "UUID"])
            .output()
            .ok()?;
        let output = String::from_utf8(output.stdout).ok()?;
        return output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.eq_ignore_ascii_case("uuid"))
            .map(str::to_lowercase);
    }
    #[allow(unreachable_code)]
    None
}

fn account_profile(account: &sdk::Account) -> AccountProfile {
    AccountProfile {
        user_id: account.user_id.clone(),
        full_name: account.full_name.clone().unwrap_or_default(),
        avatar_url: account.avatar_url.clone().unwrap_or_default(),
        identity_number: account.identity_number.clone(),
        biography: account.biography.clone(),
        phone: account.phone.clone(),
        created_at: account.created_at.clone(),
        is_verified: account.is_verified,
        fiat_currency: account.fiat_currency.clone(),
        membership: account
            .membership
            .as_ref()
            .and_then(|membership| serde_json::to_string(membership).ok()),
    }
}

async fn forward_changes(runtime: &AccountRuntime, sink: StreamSink<u64>) -> Result<()> {
    let mut changes = runtime.subscribe_conversation_changes();
    let mut shutdown = runtime.subscribe_shutdown();
    loop {
        if *shutdown.borrow() {
            break;
        }
        tokio::select! {
            result = changes.changed() => {
                if result.is_err() {
                    break;
                }
                let revision = *changes.borrow();
                if sink.add(revision).is_err() {
                    break;
                }
            }
            result = shutdown.changed() => {
                if result.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn desktop_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Linux"
    }
}
