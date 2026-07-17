use std::sync::Arc;
use std::collections::HashMap;

use anyhow::Result;
use log::warn;
use mixin_desktop_core::core::model::auth::{AuthService, AuthorizationSession};
use mixin_desktop_core::db::app::AppDatabase;
use mixin_desktop_core::db::SignalDatabase;
use mixin_desktop_core::network::{
    NetworkService, ProxyConfig, ProxySettings, ProxyType, SharedNetworkService,
};
use mixin_desktop_core::runtime::model::AccountProfile;
use mixin_desktop_core::runtime::{
    AccountRuntime, AttachmentAccess, ConversationAccess, MessageAccess, StickerAccess, UserAccess,
};
use simplelog::{Config, LevelFilter, SimpleLogger};
use tokio::sync::Mutex;

use crate::frb_generated::StreamSink;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopHandle {
    auth_service: Arc<AuthService>,
    network_service: SharedNetworkService,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct LoginHandle {
    auth_service: Arc<AuthService>,
    session: Mutex<Option<AuthorizationSession>>,
    auth_url: String,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct AccountHandle {
    runtime: Arc<AccountRuntime>,
    auth_service: Arc<AuthService>,
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

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    let _ = SimpleLogger::init(LevelFilter::Info, Config::default());
    flutter_rust_bridge::setup_default_user_utils();
}

pub async fn open_desktop() -> Result<DesktopHandle> {
    let database = Arc::new(AppDatabase::connect().await?);
    let network_service = Arc::new(NetworkService::new(database.property_dao.clone()).await?);
    let auth_service = Arc::new(AuthService::new(database));
    auth_service.initialize().await?;
    Ok(DesktopHandle {
        auth_service,
        network_service,
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
        let runtime = AccountRuntime::start(auth).await?;
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
            auth_service: self.auth_service.clone(),
        }))
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
        })
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
        let runtime = AccountRuntime::start(auth).await?;
        session.take();
        Ok(Some(AccountHandle {
            runtime: Arc::new(runtime),
            auth_service: self.auth_service.clone(),
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
        let account = self.runtime.account();
        AccountProfile {
            user_id: account.user_id.clone(),
            full_name: account.full_name.clone().unwrap_or_default(),
            avatar_url: account.avatar_url.clone().unwrap_or_default(),
            identity_number: account.identity_number.clone(),
            biography: account.biography.clone(),
            phone: account.phone.clone(),
            created_at: account.created_at.clone(),
        }
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

    pub async fn shutdown(&self) {
        self.runtime.shutdown().await;
    }

    pub async fn sign_out(&self) -> Result<()> {
        self.runtime.begin_sign_out();
        let clear_auth_error = self
            .auth_service
            .clear_auth(self.runtime.account_id())
            .await
            .err();
        self.runtime.sign_out().await;
        if let Some(error) = clear_auth_error {
            warn!("failed to clear local auth after sign out: {error}");
        }
        Ok(())
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
