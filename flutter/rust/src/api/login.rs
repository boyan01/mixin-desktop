use std::sync::Arc;

use anyhow::Result;
use mixin_desktop_core::runtime::login::LoginRuntime;

use super::account::AccountHandle;

#[flutter_rust_bridge::frb(opaque)]
pub struct LoginHandle {
    runtime: Arc<LoginRuntime>,
}

impl LoginHandle {
    pub(super) fn new(runtime: LoginRuntime) -> Self {
        Self {
            runtime: Arc::new(runtime),
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn auth_url(&self) -> String {
        self.runtime.auth_url().to_string()
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn cancel(&self) {
        self.runtime.cancel();
    }

    pub async fn wait(&self) -> Result<AccountHandle> {
        Ok(AccountHandle::new(self.runtime.wait().await?))
    }
}
