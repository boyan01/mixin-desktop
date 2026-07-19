use std::sync::Arc;

use anyhow::Result;
use mixin_desktop_core::runtime::desktop::DesktopRuntime;
use mixin_desktop_core::runtime::login::LoginRuntime;

use super::account::AccountHandle;

#[flutter_rust_bridge::frb(opaque)]
pub struct LoginHandle {
    runtime: Arc<LoginRuntime>,
    desktop: Arc<DesktopRuntime>,
}

impl LoginHandle {
    pub(super) fn new(runtime: LoginRuntime, desktop: Arc<DesktopRuntime>) -> Self {
        Self {
            runtime: Arc::new(runtime),
            desktop,
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
        Ok(AccountHandle::new(
            self.desktop.wait_login(&self.runtime).await?,
            self.desktop.clone(),
        ))
    }
}
