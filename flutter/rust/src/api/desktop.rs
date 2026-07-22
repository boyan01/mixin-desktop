use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use mixin_desktop_core::network::{ProxyConfig, ProxySettings, ProxyType};
use mixin_desktop_core::runtime::desktop::DesktopRuntime;
use mixin_desktop_core::runtime::mcp::{McpServerStatus, McpSettings};
use tokio::sync::OnceCell;

use super::account::AccountHandle;
use super::login::LoginHandle;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopHandle {
    runtime: Arc<DesktopRuntime>,
}

static DESKTOP_RUNTIME: OnceCell<Arc<DesktopRuntime>> = OnceCell::const_new();

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

pub struct McpSettingsItem {
    pub enabled: bool,
    pub token: String,
    pub draft_tools_enabled: bool,
    pub circle_management_enabled: bool,
}

pub struct McpServerStatusItem {
    pub running: bool,
    pub endpoint: Option<String>,
    pub last_error: Option<String>,
}

impl From<McpSettings> for McpSettingsItem {
    fn from(settings: McpSettings) -> Self {
        Self {
            enabled: settings.enabled,
            token: settings.token,
            draft_tools_enabled: settings.draft_tools_enabled,
            circle_management_enabled: settings.circle_management_enabled,
        }
    }
}

impl From<McpServerStatus> for McpServerStatusItem {
    fn from(status: McpServerStatus) -> Self {
        Self {
            running: status.running,
            endpoint: status.endpoint,
            last_error: status.last_error,
        }
    }
}

impl From<McpSettingsItem> for McpSettings {
    fn from(settings: McpSettingsItem) -> Self {
        Self {
            enabled: settings.enabled,
            token: settings.token,
            draft_tools_enabled: settings.draft_tools_enabled,
            circle_management_enabled: settings.circle_management_enabled,
        }
    }
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

pub async fn open_desktop() -> Result<DesktopHandle> {
    let runtime = DESKTOP_RUNTIME
        .get_or_try_init(|| async {
            Ok::<_, anyhow::Error>(Arc::new(DesktopRuntime::open().await?))
        })
        .await?;
    Ok(DesktopHandle {
        runtime: runtime.clone(),
    })
}

impl DesktopHandle {
    pub async fn mcp_settings(&self) -> Result<McpSettingsItem> {
        Ok(self.runtime.mcp_settings().await?.into())
    }

    pub async fn update_mcp_settings(
        &self,
        settings: McpSettingsItem,
    ) -> Result<McpServerStatusItem> {
        Ok(self
            .runtime
            .update_mcp_settings(settings.into())
            .await?
            .into())
    }

    pub async fn mcp_server_status(&self) -> Result<McpServerStatusItem> {
        Ok(self.runtime.mcp_server_status().await.into())
    }

    pub async fn proxy_settings(&self) -> Result<ProxySettingsItem> {
        Ok(self.runtime.proxy_settings().await.into())
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettingsItem) -> Result<()> {
        self.runtime.set_proxy_settings(settings.try_into()?).await
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
            .runtime
            .http_request(
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
        Ok(self
            .runtime
            .restore_account()
            .await?
            .map(|account| AccountHandle::new(account, self.runtime.clone())))
    }

    pub async fn recreate_account_database(&self) -> Result<()> {
        self.runtime.recreate_account_database().await
    }

    pub async fn abort_saved_login(&self) -> Result<()> {
        self.runtime.abort_saved_login().await
    }

    pub async fn begin_login(&self) -> Result<LoginHandle> {
        Ok(LoginHandle::new(
            self.runtime.begin_login().await?,
            self.runtime.clone(),
        ))
    }
}
