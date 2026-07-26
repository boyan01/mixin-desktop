use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt as _;
use mixin_desktop_api::{
    DesktopClient, HttpResponseItem, McpServerStatusItem, McpSettingsItem, ProxySettingsItem,
    SettingsClient,
};

use crate::{frb_generated::StreamSink, CoreError, Result};

use super::account::AccountHandle;
use super::login::LoginHandle;
use super::media::MediaHandle;

#[flutter_rust_bridge::frb(opaque)]
pub struct DesktopHandle {
    client: Arc<DesktopClient>,
}

#[flutter_rust_bridge::frb(opaque)]
pub struct SettingsHandle {
    client: Arc<SettingsClient>,
}

pub async fn open_desktop() -> Result<DesktopHandle, CoreError> {
    Ok(DesktopHandle {
        client: Arc::new(DesktopClient::open().await?),
    })
}

impl DesktopHandle {
    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn settings(&self) -> SettingsHandle {
        SettingsHandle {
            client: Arc::new(self.client.settings()),
        }
    }

    #[flutter_rust_bridge::frb(sync, getter)]
    pub fn media(&self) -> MediaHandle {
        MediaHandle::new(self.client.media())
    }

    pub async fn http_request(
        &self,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        timeout_millis: Option<u64>,
        max_response_bytes: Option<u64>,
    ) -> Result<HttpResponseItem, CoreError> {
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
            .await?)
    }

    pub async fn recreate_account_database(&self) -> Result<(), CoreError> {
        Ok(self.client.recreate_account_database().await?)
    }

    pub async fn abort_saved_login(&self) -> Result<(), CoreError> {
        Ok(self.client.abort_saved_login().await?)
    }

    pub async fn begin_login(&self) -> Result<LoginHandle, CoreError> {
        Ok(LoginHandle::new(self.client.begin_login().await?))
    }

    pub async fn restore_account(&self) -> Result<AccountHandle, CoreError> {
        Ok(AccountHandle::new(self.client.restore_account().await?))
    }
}

impl SettingsHandle {
    pub async fn photo_auto_download(&self) -> Result<bool, CoreError> {
        Ok(self.client.photo_auto_download().await?)
    }

    pub async fn set_photo_auto_download(&self, value: bool) -> Result<(), CoreError> {
        Ok(self.client.set_photo_auto_download(value).await?)
    }
}

impl SettingsHandle {
    pub async fn subscribe_photo_auto_download(
        &self,
        sink: StreamSink<bool>,
    ) -> Result<(), CoreError> {
        let subscription = self.client.subscribe_photo_auto_download();
        futures::pin_mut!(subscription);
        while let Some(value) = subscription.next().await {
            if sink.add(value?).is_err() {
                return Ok(());
            }
        }
        Ok(())
    }
}

impl SettingsHandle {
    pub async fn video_auto_download(&self) -> Result<bool, CoreError> {
        Ok(self.client.video_auto_download().await?)
    }

    pub async fn set_video_auto_download(&self, value: bool) -> Result<(), CoreError> {
        Ok(self.client.set_video_auto_download(value).await?)
    }
}

impl SettingsHandle {
    pub async fn subscribe_video_auto_download(
        &self,
        sink: StreamSink<bool>,
    ) -> Result<(), CoreError> {
        let subscription = self.client.subscribe_video_auto_download();
        futures::pin_mut!(subscription);
        while let Some(value) = subscription.next().await {
            if sink.add(value?).is_err() {
                return Ok(());
            }
        }
        Ok(())
    }
}

impl SettingsHandle {
    pub async fn file_auto_download(&self) -> Result<bool, CoreError> {
        Ok(self.client.file_auto_download().await?)
    }

    pub async fn set_file_auto_download(&self, value: bool) -> Result<(), CoreError> {
        Ok(self.client.set_file_auto_download(value).await?)
    }
}

impl SettingsHandle {
    pub async fn subscribe_file_auto_download(
        &self,
        sink: StreamSink<bool>,
    ) -> Result<(), CoreError> {
        let subscription = self.client.subscribe_file_auto_download();
        futures::pin_mut!(subscription);
        while let Some(value) = subscription.next().await {
            if sink.add(value?).is_err() {
                return Ok(());
            }
        }
        Ok(())
    }
}

impl SettingsHandle {
    pub async fn setting(&self, key: String) -> Result<Option<String>, CoreError> {
        Ok(self.client.setting(key).await?)
    }

    pub async fn set_setting(&self, key: String, value: Option<String>) -> Result<(), CoreError> {
        Ok(self.client.set_setting(key, value).await?)
    }
}

impl SettingsHandle {
    pub async fn subscribe_setting(
        &self,
        key: String,
        sink: StreamSink<Option<String>>,
    ) -> Result<(), CoreError> {
        let subscription = self.client.subscribe_setting(key);
        futures::pin_mut!(subscription);
        while let Some(value) = subscription.next().await {
            if sink.add(value?).is_err() {
                return Ok(());
            }
        }
        Ok(())
    }
}

impl SettingsHandle {
    pub async fn mcp_settings(&self) -> Result<McpSettingsItem, CoreError> {
        Ok(self.client.mcp_settings().await?)
    }

    pub async fn update_mcp_settings(
        &self,
        settings: McpSettingsItem,
    ) -> Result<McpServerStatusItem, CoreError> {
        Ok(self.client.update_mcp_settings(settings).await?)
    }

    pub async fn mcp_server_status(&self) -> Result<McpServerStatusItem, CoreError> {
        Ok(self.client.mcp_server_status().await?)
    }

    pub async fn proxy_settings(&self) -> Result<ProxySettingsItem, CoreError> {
        Ok(self.client.proxy_settings().await?)
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettingsItem) -> Result<(), CoreError> {
        Ok(self.client.set_proxy_settings(settings).await?)
    }
}
