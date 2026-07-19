use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use mixin_desktop_core::network::{ProxyConfig, ProxySettings, ProxyType};
use mixin_desktop_core::runtime::desktop::DesktopRuntime;

use super::account::AccountHandle;
use super::login::LoginHandle;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopHandle {
    runtime: Arc<DesktopRuntime>,
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

pub async fn open_desktop() -> Result<DesktopHandle> {
    Ok(DesktopHandle {
        runtime: Arc::new(DesktopRuntime::open().await?),
    })
}

impl DesktopHandle {
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
            .map(AccountHandle::new))
    }

    pub async fn recreate_account_database(&self) -> Result<()> {
        self.runtime.recreate_account_database().await
    }

    pub async fn abort_saved_login(&self) -> Result<()> {
        self.runtime.abort_saved_login().await
    }

    pub async fn begin_login(&self) -> Result<LoginHandle> {
        Ok(LoginHandle::new(self.runtime.begin_login().await?))
    }
}
