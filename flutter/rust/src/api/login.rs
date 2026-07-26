use std::sync::Arc;

use mixin_desktop_api::LoginClient;

use super::account::AccountHandle;
use crate::{CoreError, Result};

#[flutter_rust_bridge::frb(opaque)]
pub struct LoginHandle {
    client: Arc<LoginClient>,
}

impl LoginHandle {
    pub(super) fn new(client: LoginClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

impl LoginHandle {
    #[flutter_rust_bridge::frb(sync)]
    pub fn auth_url(&self) -> String {
        self.client.auth_url()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        self.client.cancel();
    }

    pub async fn wait(&self) -> Result<AccountHandle, CoreError> {
        Ok(AccountHandle::new(self.client.wait().await?))
    }
}
