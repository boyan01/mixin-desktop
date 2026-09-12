use std::sync::Arc;

use mixin_desktop_api::LoginClient;

use crate::{account::SwiftAccountHandle, error::SwiftClientError};

#[derive(uniffi::Object)]
pub struct SwiftLoginHandle {
    client: Arc<LoginClient>,
}

impl SwiftLoginHandle {
    pub(crate) fn new(client: LoginClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftLoginHandle {
    pub async fn wait(&self) -> Result<SwiftAccountHandle, SwiftClientError> {
        Ok(SwiftAccountHandle::new(self.client.wait().await?))
    }
}

include!("generated/login.rs");
