use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::{watch, Mutex};

use crate::core::model::auth::{AuthService, AuthorizationSession};
use crate::db::app::PropertyDao;

use super::desktop::record_current_device;
use super::AccountRuntime;

pub struct LoginRuntime {
    auth_service: Arc<AuthService>,
    session: Mutex<Option<AuthorizationSession>>,
    auth_url: String,
    property_dao: PropertyDao,
    cancelled: watch::Sender<bool>,
}

impl LoginRuntime {
    pub(super) async fn start(
        auth_service: Arc<AuthService>,
        property_dao: PropertyDao,
    ) -> Result<Self> {
        let session = auth_service.begin_authorization(desktop_platform()).await?;
        let auth_url = session.auth_url().to_string();
        let (cancelled, _) = watch::channel(false);
        Ok(Self {
            auth_service,
            session: Mutex::new(Some(session)),
            auth_url,
            property_dao,
            cancelled,
        })
    }

    pub fn auth_url(&self) -> &str {
        &self.auth_url
    }

    pub fn cancel(&self) {
        self.cancelled.send_replace(true);
    }

    pub async fn wait(&self) -> Result<AccountRuntime> {
        let mut session = self.session.lock().await;
        let active_session = session
            .take()
            .ok_or_else(|| anyhow::anyhow!("login is no longer active"))?;
        drop(session);

        let mut cancelled = self.cancelled.subscribe();
        if *cancelled.borrow() {
            anyhow::bail!("login cancelled");
        }
        let result = tokio::select! {
            result = self.auth_service.wait_authorization(
                &active_session,
                Duration::from_secs(60),
            ) => result?,
            _ = cancelled.changed() => anyhow::bail!("login cancelled"),
        };
        let auth = self.auth_service.complete_authorization(result).await?;
        record_current_device(&self.property_dao).await?;
        AccountRuntime::start(auth, self.auth_service.clone())
            .await
            .map_err(|error| anyhow::anyhow!("login_provisioning_error:{error}"))
    }
}

fn desktop_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Linux"
    }
}
