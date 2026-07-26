use std::sync::Arc;

use mixin_desktop_api::DesktopClient;

use crate::{account::SwiftAccountHandle, error::SwiftClientError, login::SwiftLoginHandle};

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
    pub async fn restore_account(&self) -> Result<SwiftAccountHandle, SwiftClientError> {
        Ok(SwiftAccountHandle::new(
            self.client.restore_account().await?,
        ))
    }

    pub async fn begin_login(&self) -> Result<SwiftLoginHandle, SwiftClientError> {
        Ok(SwiftLoginHandle::new(self.client.begin_login().await?))
    }
}
