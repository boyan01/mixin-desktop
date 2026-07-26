use std::sync::Arc;

use mixin_desktop_core::runtime::{desktop::DesktopRuntime, login::LoginRuntime};

use crate::{AccountClient, ClientResult};

pub struct LoginClient {
    runtime: LoginRuntime,
    desktop: Arc<DesktopRuntime>,
}

impl LoginClient {
    pub(crate) fn new(runtime: LoginRuntime, desktop: Arc<DesktopRuntime>) -> Self {
        Self { runtime, desktop }
    }

    pub fn auth_url(&self) -> String {
        self.runtime.auth_url().to_string()
    }

    pub fn cancel(&self) {
        self.runtime.cancel();
    }

    pub async fn wait(&self) -> ClientResult<AccountClient> {
        Ok(AccountClient::new(
            self.desktop.wait_login(&self.runtime).await?,
            self.desktop.clone(),
        ))
    }
}
