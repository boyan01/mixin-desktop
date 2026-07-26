use std::sync::Arc;

use mixin_desktop_api::DesktopClient;

use crate::{
    account::SwiftAccountHandle,
    error::SwiftClientError,
    login::SwiftLoginHandle,
    media::SwiftMediaHandle,
    model::{SwiftHttpResponse, SwiftMcpServerStatus, SwiftMcpSettings, SwiftProxySettings},
};

#[derive(uniffi::Object)]
pub struct SwiftDesktopHandle {
    client: Arc<DesktopClient>,
}

#[uniffi::export(async_runtime = "tokio")]
pub async fn open_desktop() -> Result<SwiftDesktopHandle, SwiftClientError> {
    Ok(SwiftDesktopHandle {
        client: Arc::new(DesktopClient::open().await?),
    })
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftDesktopHandle {
    pub fn media(&self) -> SwiftMediaHandle {
        SwiftMediaHandle::new(self.client.media())
    }

    pub async fn http_request(
        &self,
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        body: Option<Vec<u8>>,
        timeout_millis: Option<u64>,
        max_response_bytes: Option<u64>,
    ) -> Result<SwiftHttpResponse, SwiftClientError> {
        Ok(self
            .client
            .http_request(
                method,
                url,
                headers,
                body,
                timeout_millis,
                max_response_bytes,
            )
            .await?
            .into())
    }

    pub async fn restore_account(&self) -> Result<SwiftAccountHandle, SwiftClientError> {
        Ok(SwiftAccountHandle::new(
            self.client.restore_account().await?,
        ))
    }

    pub async fn begin_login(&self) -> Result<SwiftLoginHandle, SwiftClientError> {
        Ok(SwiftLoginHandle::new(self.client.begin_login().await?))
    }

    pub async fn recreate_account_database(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.recreate_account_database().await?)
    }

    pub async fn abort_saved_login(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.abort_saved_login().await?)
    }

    pub async fn setting(&self, key: String) -> Result<Option<String>, SwiftClientError> {
        Ok(self.client.settings().setting(key).await?)
    }

    pub async fn set_setting(
        &self,
        key: String,
        value: Option<String>,
    ) -> Result<(), SwiftClientError> {
        Ok(self.client.settings().set_setting(key, value).await?)
    }

    pub async fn photo_auto_download(&self) -> Result<bool, SwiftClientError> {
        Ok(self.client.settings().photo_auto_download().await?)
    }

    pub async fn set_photo_auto_download(&self, value: bool) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .settings()
            .set_photo_auto_download(value)
            .await?)
    }

    pub async fn video_auto_download(&self) -> Result<bool, SwiftClientError> {
        Ok(self.client.settings().video_auto_download().await?)
    }

    pub async fn set_video_auto_download(&self, value: bool) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .settings()
            .set_video_auto_download(value)
            .await?)
    }

    pub async fn file_auto_download(&self) -> Result<bool, SwiftClientError> {
        Ok(self.client.settings().file_auto_download().await?)
    }

    pub async fn set_file_auto_download(&self, value: bool) -> Result<(), SwiftClientError> {
        Ok(self.client.settings().set_file_auto_download(value).await?)
    }

    pub async fn proxy_settings(&self) -> Result<SwiftProxySettings, SwiftClientError> {
        Ok(self.client.settings().proxy_settings().await?.into())
    }

    pub async fn set_proxy_settings(
        &self,
        settings: SwiftProxySettings,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .settings()
            .set_proxy_settings(settings.into())
            .await?)
    }

    pub async fn mcp_settings(&self) -> Result<SwiftMcpSettings, SwiftClientError> {
        Ok(self.client.settings().mcp_settings().await?.into())
    }

    pub async fn update_mcp_settings(
        &self,
        settings: SwiftMcpSettings,
    ) -> Result<SwiftMcpServerStatus, SwiftClientError> {
        Ok(self
            .client
            .settings()
            .update_mcp_settings(settings.into())
            .await?
            .into())
    }

    pub async fn mcp_server_status(&self) -> Result<SwiftMcpServerStatus, SwiftClientError> {
        Ok(self.client.settings().mcp_server_status().await?.into())
    }

    pub fn log_directory(&self) -> Result<String, SwiftClientError> {
        Ok(mixin_desktop_api::log_directory()?)
    }
}
