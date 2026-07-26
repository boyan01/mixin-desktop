use std::collections::HashMap;
use std::sync::Arc;

use futures::{Stream, StreamExt as _};
use mixin_desktop_core::network::ProxySettings;
use mixin_desktop_core::runtime::{desktop::DesktopRuntime, logging};
use tokio::sync::OnceCell;

use crate::{
    AccountClient, ClientResult, HttpResponseItem, LoginClient, McpServerStatusItem,
    McpSettingsItem, MediaClient, ProxySettingsItem,
};

#[derive(Clone)]
pub struct DesktopClient {
    runtime: Arc<DesktopRuntime>,
    media: Arc<MediaClient>,
}

#[derive(Clone)]
pub struct SettingsClient {
    runtime: Arc<DesktopRuntime>,
}

static DESKTOP_RUNTIME: OnceCell<Arc<DesktopRuntime>> = OnceCell::const_new();
static MEDIA_CLIENT: std::sync::OnceLock<Arc<MediaClient>> = std::sync::OnceLock::new();

impl DesktopClient {
    pub async fn open() -> ClientResult<Self> {
        logging::init(
            "Mixin".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        )
        .map_err(|error| crate::ClientError::Internal(error.to_string()))?;
        let runtime = DESKTOP_RUNTIME
            .get_or_try_init(|| async {
                Ok::<_, crate::ClientError>(Arc::new(DesktopRuntime::open().await?))
            })
            .await?;
        Ok(Self {
            runtime: runtime.clone(),
            media: MEDIA_CLIENT
                .get_or_init(|| Arc::new(MediaClient::new()))
                .clone(),
        })
    }

    pub async fn restore_account(&self) -> ClientResult<AccountClient> {
        let runtime = self.runtime.restore_account().await?;
        let runtime = runtime.ok_or(mixin_desktop_core::CoreError::NotFound)?;
        Ok(AccountClient::new(runtime, self.runtime.clone()))
    }

    pub async fn begin_login(&self) -> ClientResult<LoginClient> {
        Ok(LoginClient::new(
            self.runtime.begin_login().await?,
            self.runtime.clone(),
        ))
    }

    pub fn settings(&self) -> SettingsClient {
        SettingsClient {
            runtime: self.runtime.clone(),
        }
    }

    pub fn media(&self) -> Arc<MediaClient> {
        self.media.clone()
    }

    pub async fn http_request(
        &self,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        timeout_millis: Option<u64>,
        max_response_bytes: Option<u64>,
    ) -> ClientResult<HttpResponseItem> {
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

    pub async fn recreate_account_database(&self) -> ClientResult<()> {
        Ok(self.runtime.recreate_account_database().await?)
    }

    pub async fn abort_saved_login(&self) -> ClientResult<()> {
        Ok(self.runtime.abort_saved_login().await?)
    }
}

impl SettingsClient {
    pub async fn photo_auto_download(&self) -> ClientResult<bool> {
        Ok(self.runtime.settings.photo_auto_download().await?)
    }

    pub async fn set_photo_auto_download(&self, value: bool) -> ClientResult<()> {
        Ok(self.runtime.settings.set_photo_auto_download(value).await?)
    }

    pub fn subscribe_photo_auto_download(
        &self,
    ) -> impl Stream<Item = ClientResult<bool>> + Send + 'static {
        self.runtime
            .settings
            .subscribe_photo_auto_download()
            .map(|result| result.map_err(Into::into))
    }

    pub async fn video_auto_download(&self) -> ClientResult<bool> {
        Ok(self.runtime.settings.video_auto_download().await?)
    }

    pub async fn set_video_auto_download(&self, value: bool) -> ClientResult<()> {
        Ok(self.runtime.settings.set_video_auto_download(value).await?)
    }

    pub fn subscribe_video_auto_download(
        &self,
    ) -> impl Stream<Item = ClientResult<bool>> + Send + 'static {
        self.runtime
            .settings
            .subscribe_video_auto_download()
            .map(|result| result.map_err(Into::into))
    }

    pub async fn file_auto_download(&self) -> ClientResult<bool> {
        Ok(self.runtime.settings.file_auto_download().await?)
    }

    pub async fn set_file_auto_download(&self, value: bool) -> ClientResult<()> {
        Ok(self.runtime.settings.set_file_auto_download(value).await?)
    }

    pub fn subscribe_file_auto_download(
        &self,
    ) -> impl Stream<Item = ClientResult<bool>> + Send + 'static {
        self.runtime
            .settings
            .subscribe_file_auto_download()
            .map(|result| result.map_err(Into::into))
    }

    pub async fn setting(&self, key: String) -> ClientResult<Option<String>> {
        Ok(self.runtime.settings.get(&key).await?)
    }

    pub async fn set_setting(&self, key: String, value: Option<String>) -> ClientResult<()> {
        Ok(self.runtime.settings.set(&key, value.as_deref()).await?)
    }

    pub fn subscribe_setting(
        &self,
        key: String,
    ) -> impl Stream<Item = ClientResult<Option<String>>> + Send + 'static {
        self.runtime
            .settings
            .subscribe(key)
            .map(|result| result.map_err(Into::into))
    }

    pub async fn mcp_settings(&self) -> ClientResult<McpSettingsItem> {
        Ok(self.runtime.mcp_server.settings().await?.into())
    }

    pub async fn update_mcp_settings(
        &self,
        settings: McpSettingsItem,
    ) -> ClientResult<McpServerStatusItem> {
        Ok(self
            .runtime
            .mcp_server
            .update_settings(settings.into())
            .await?
            .into())
    }

    pub async fn mcp_server_status(&self) -> ClientResult<McpServerStatusItem> {
        Ok(self.runtime.mcp_server.status().await.into())
    }

    pub async fn proxy_settings(&self) -> ClientResult<ProxySettingsItem> {
        Ok(self.runtime.settings.proxy_settings().await?.into())
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettingsItem) -> ClientResult<()> {
        let settings: ProxySettings = settings.try_into()?;
        settings.validate()?;
        Ok(self.runtime.settings.set_proxy_settings(settings).await?)
    }
}
