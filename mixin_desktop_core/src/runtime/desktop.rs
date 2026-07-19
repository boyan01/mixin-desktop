use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Local;
use log::warn;
use sdk::Client;

use crate::core::model::auth::AuthService;
use crate::db::app::{AppDatabase, PropertyDao};
use crate::db::path::account_data_directory;
use crate::db::SignalDatabase;
use crate::network::{HttpResponse, NetworkService, ProxySettings, SharedNetworkService};

use super::login::LoginRuntime;
use super::{credential, AccountRuntime};

const DEVICE_PROPERTY_GROUP: &str = "account";
const DEVICE_PROPERTY_KEY: &str = "device_id";

pub struct DesktopRuntime {
    auth_service: Arc<AuthService>,
    network_service: SharedNetworkService,
    property_dao: PropertyDao,
}

impl DesktopRuntime {
    pub async fn open() -> Result<Self> {
        let database = Arc::new(AppDatabase::connect().await?);
        let network_service = Arc::new(NetworkService::new(database.property_dao.clone()).await?);
        let auth_service = Arc::new(AuthService::new(database.clone()));
        auth_service.initialize().await?;
        Ok(Self {
            auth_service,
            network_service,
            property_dao: database.property_dao.clone(),
        })
    }

    pub async fn proxy_settings(&self) -> ProxySettings {
        self.network_service.proxy_settings().await
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettings) -> Result<()> {
        self.network_service.set_proxy_settings(settings).await
    }

    pub async fn http_request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
        timeout_millis: Option<u64>,
        max_response_bytes: Option<usize>,
    ) -> Result<HttpResponse> {
        self.network_service
            .request(
                method,
                url,
                headers,
                body,
                timeout_millis,
                max_response_bytes,
            )
            .await
    }

    pub async fn restore_account(&self) -> Result<Option<AccountRuntime>> {
        let Some(auth) = self.auth_service.get_auth() else {
            return Ok(None);
        };
        if !device_matches(
            &self.property_dao,
            &self.auth_service,
            &auth.account.user_id,
            &auth.account.identity_number,
        )
        .await?
        {
            return Ok(None);
        }
        let signal_database = SignalDatabase::connect(auth.account.identity_number.clone())
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        if signal_database
            .identity_dao
            .get_local_identity()
            .await?
            .is_none()
        {
            self.auth_service.clear_auth(&auth.account.user_id).await?;
            return Ok(None);
        }
        Ok(Some(
            AccountRuntime::start(auth, self.auth_service.clone()).await?,
        ))
    }

    pub async fn recreate_account_database(&self) -> Result<()> {
        let auth = self
            .auth_service
            .get_auth()
            .ok_or_else(|| anyhow!("no saved account"))?;
        let database = account_data_directory(&auth.account.identity_number)?.join("mixin.db");
        rename_with_timestamp_if_exists(&database).await?;
        remove_if_exists(&database.with_extension("db-shm")).await?;
        remove_if_exists(&database.with_extension("db-wal")).await?;
        Ok(())
    }

    pub async fn abort_saved_login(&self) -> Result<()> {
        let Some(auth) = self.auth_service.get_auth() else {
            return Ok(());
        };
        let client = Client::new(credential(&auth));
        if let Err(error) = client.account_api.logout(&auth.account.session_id).await {
            warn!("failed to revoke session after login startup failure: {error}");
        }
        self.auth_service.clear_auth(&auth.account.user_id).await?;
        let database = match SignalDatabase::connect(auth.account.identity_number).await {
            Ok(database) => database,
            Err(error) => {
                warn!("failed to open signal state after login startup failure: {error}");
                return Ok(());
            }
        };
        if let Err(error) = database.clear().await {
            warn!("failed to clear signal state after login startup failure: {error}");
        }
        database.close().await;
        Ok(())
    }

    pub async fn begin_login(&self) -> Result<LoginRuntime> {
        LoginRuntime::start(self.auth_service.clone(), self.property_dao.clone()).await
    }
}

async fn rename_with_timestamp_if_exists(path: &Path) -> Result<()> {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(anyhow!("database path has no file name"));
    };
    let target = path.with_file_name(format!("{file_name}.{}", Local::now().to_rfc3339()));
    match tokio::fs::rename(path, target).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn remove_if_exists(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn device_matches(
    property_dao: &PropertyDao,
    auth_service: &AuthService,
    user_id: &str,
    identity_number: &str,
) -> Result<bool> {
    let Some(current) = current_device_id().await else {
        return Ok(true);
    };
    let saved = property_dao
        .get(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY)
        .await?;
    if saved
        .as_deref()
        .is_none_or(|saved| saved.eq_ignore_ascii_case(&current))
    {
        property_dao
            .set(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY, &current)
            .await?;
        return Ok(true);
    }

    auth_service.clear_auth(user_id).await?;
    let directory = account_data_directory(identity_number)?;
    match tokio::fs::remove_dir_all(&directory).await {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    property_dao
        .set(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY, &current)
        .await?;
    Ok(false)
}

pub(super) async fn record_current_device(property_dao: &PropertyDao) -> Result<()> {
    let Some(current) = current_device_id().await else {
        return Ok(());
    };
    property_dao
        .set(DEVICE_PROPERTY_GROUP, DEVICE_PROPERTY_KEY, &current)
        .await?;
    Ok(())
}

async fn current_device_id() -> Option<String> {
    tokio::task::spawn_blocking(current_device_id_sync)
        .await
        .ok()
        .flatten()
}

fn current_device_id_sync() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let id = std::fs::read_to_string("/etc/machine-id").ok()?;
        let id = id.trim();
        return (id.len() == 32 && id.chars().all(|character| character.is_ascii_hexdigit()))
            .then(|| id.to_lowercase());
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok()?;
        let output = String::from_utf8(output.stdout).ok()?;
        return output.lines().find_map(|line| {
            if !line.contains("IOPlatformUUID") {
                return None;
            }
            line.split('=')
                .nth(1)
                .map(|value| value.trim().trim_matches('"').to_lowercase())
                .filter(|value| !value.is_empty())
        });
    }
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("wmic")
            .args(["csproduct", "get", "UUID"])
            .output()
            .ok()?;
        let output = String::from_utf8(output.stdout).ok()?;
        return output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.eq_ignore_ascii_case("uuid"))
            .map(str::to_lowercase);
    }
    #[allow(unreachable_code)]
    None
}
