use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Local;
use log::{info, warn};
use sdk::Client;
use tokio::sync::Mutex;

use crate::core::model::auth::AuthService;
use crate::core::user_agent::generate_user_agent;
use crate::db::app::{AppDatabase, PropertyDao};
use crate::db::path::account_data_directory;
use crate::db::SignalDatabase;
use crate::network::{HttpResponse, NetworkService, ProxySettings, SharedNetworkService};

use super::login::LoginRuntime;
use super::mcp::{generate_access_token, McpServer, McpServerStatus, McpSettings};
use super::{credential, AccountRuntime, SessionUnauthorized};

const DEVICE_PROPERTY_GROUP: &str = "account";
const DEVICE_PROPERTY_KEY: &str = "device_id";
const MCP_PROPERTY_GROUP: &str = "mcp";
const MCP_ENABLED_KEY: &str = "enabled";
const MCP_TOKEN_KEY: &str = "token";
const MCP_DRAFT_TOOLS_KEY: &str = "draft_tools_enabled";
const MCP_CIRCLE_TOOLS_KEY: &str = "circle_management_enabled";

pub struct DesktopRuntime {
    auth_service: Arc<AuthService>,
    network_service: SharedNetworkService,
    property_dao: PropertyDao,
    account: Mutex<Option<Arc<AccountRuntime>>>,
    mcp: Mutex<McpState>,
}

#[derive(Default)]
struct McpState {
    server: Option<McpServer>,
    last_error: Option<String>,
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
            account: Mutex::new(None),
            mcp: Mutex::new(McpState::default()),
        })
    }

    pub async fn proxy_settings(&self) -> ProxySettings {
        self.network_service.proxy_settings().await
    }

    pub async fn set_proxy_settings(&self, settings: ProxySettings) -> Result<()> {
        self.network_service.set_proxy_settings(settings).await
    }

    pub async fn mcp_settings(&self) -> Result<McpSettings> {
        let token = match self
            .property_dao
            .get(MCP_PROPERTY_GROUP, MCP_TOKEN_KEY)
            .await?
        {
            Some(token) if !token.is_empty() => token,
            _ => {
                let token = generate_access_token();
                self.property_dao
                    .set(MCP_PROPERTY_GROUP, MCP_TOKEN_KEY, &token)
                    .await?;
                token
            }
        };
        Ok(McpSettings {
            enabled: property_bool(&self.property_dao, MCP_ENABLED_KEY).await?,
            token,
            draft_tools_enabled: property_bool(&self.property_dao, MCP_DRAFT_TOOLS_KEY).await?,
            circle_management_enabled: property_bool(&self.property_dao, MCP_CIRCLE_TOOLS_KEY)
                .await?,
        })
    }

    pub async fn update_mcp_settings(&self, settings: McpSettings) -> Result<McpServerStatus> {
        self.property_dao
            .update(&[
                (
                    MCP_PROPERTY_GROUP,
                    MCP_ENABLED_KEY,
                    Some(bool_property(settings.enabled).as_str()),
                ),
                (
                    MCP_PROPERTY_GROUP,
                    MCP_TOKEN_KEY,
                    Some(settings.token.as_str()),
                ),
                (
                    MCP_PROPERTY_GROUP,
                    MCP_DRAFT_TOOLS_KEY,
                    Some(bool_property(settings.draft_tools_enabled).as_str()),
                ),
                (
                    MCP_PROPERTY_GROUP,
                    MCP_CIRCLE_TOOLS_KEY,
                    Some(bool_property(settings.circle_management_enabled).as_str()),
                ),
            ])
            .await?;
        self.sync_mcp_server().await
    }

    pub async fn mcp_server_status(&self) -> McpServerStatus {
        let state = self.mcp.lock().await;
        McpServerStatus {
            running: state.server.is_some(),
            endpoint: state
                .server
                .as_ref()
                .map(|server| server.endpoint().to_owned()),
            last_error: state.last_error.clone(),
        }
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

    pub async fn restore_account(&self) -> Result<Option<Arc<AccountRuntime>>> {
        let mut active = self.account.lock().await;
        if let Some(runtime) = active.as_ref() {
            if runtime.is_running() {
                info!(
                    "reusing active account runtime for {}",
                    runtime.account_id()
                );
                return Ok(Some(runtime.clone()));
            }
            let runtime = active.take().unwrap();
            runtime.shutdown().await;
        }
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
        let user_id = auth.account.user_id.clone();
        match AccountRuntime::start(auth, self.auth_service.clone()).await {
            Ok(runtime) => {
                let runtime = Arc::new(runtime);
                info!("started account runtime for {}", runtime.account_id());
                *active = Some(runtime.clone());
                drop(active);
                self.sync_mcp_server().await?;
                Ok(Some(runtime))
            }
            Err(error) if error.downcast_ref::<SessionUnauthorized>().is_some() => {
                warn!("saved session is unauthorized; requiring login");
                self.auth_service.clear_auth(&user_id).await?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub async fn recreate_account_database(&self) -> Result<()> {
        self.shutdown_active_account().await;
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
        let client = Client::new_with_user_agent(credential(&auth), Some(generate_user_agent()));
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
        if self.account.lock().await.is_some() {
            return Err(anyhow!("account runtime already active"));
        }
        LoginRuntime::start(self.auth_service.clone(), self.property_dao.clone()).await
    }

    pub async fn wait_login(&self, login: &LoginRuntime) -> Result<Arc<AccountRuntime>> {
        let auth = login.wait_authorization().await?;
        let mut active = self.account.lock().await;
        if let Some(runtime) = active.as_ref() {
            if runtime.is_running() && runtime.account_id() == auth.account.user_id {
                return Ok(runtime.clone());
            }
            let runtime = active.take().unwrap();
            runtime.shutdown().await;
        }
        let runtime = Arc::new(
            AccountRuntime::start(auth, self.auth_service.clone())
                .await
                .map_err(|error| anyhow!("login_provisioning_error:{error}"))?,
        );
        *active = Some(runtime.clone());
        drop(active);
        self.sync_mcp_server().await?;
        Ok(runtime)
    }

    pub async fn shutdown_account(&self, runtime: &Arc<AccountRuntime>) {
        let mut active = self.account.lock().await;
        let was_active = active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, runtime));
        if was_active {
            active.take();
            self.stop_mcp_server().await;
        }
        runtime.shutdown().await;
    }

    pub async fn sign_out_account(&self, runtime: &Arc<AccountRuntime>) -> Result<()> {
        let mut active = self.account.lock().await;
        if !active
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, runtime))
        {
            return Err(anyhow!("account runtime is no longer active"));
        }
        self.stop_mcp_server().await;
        let result = runtime.sign_out().await;
        active.take();
        if result.is_err() {
            runtime.shutdown().await;
        }
        result
    }

    async fn shutdown_active_account(&self) {
        let mut active = self.account.lock().await;
        let runtime = active.take();
        self.stop_mcp_server().await;
        if let Some(runtime) = runtime {
            runtime.shutdown().await;
        }
        drop(active);
    }

    async fn sync_mcp_server(&self) -> Result<McpServerStatus> {
        let settings = self.mcp_settings().await?;
        let runtime = self.account.lock().await.clone();
        let mut state = self.mcp.lock().await;
        if let Some(server) = state.server.take() {
            server.stop().await;
        }
        state.last_error = None;
        if settings.enabled {
            let runtime = runtime.ok_or_else(|| anyhow!("MCP requires a signed-in account"))?;
            match McpServer::start(runtime, settings).await {
                Ok(server) => state.server = Some(server),
                Err(error) => {
                    state.last_error = Some(error.to_string());
                    return Ok(McpServerStatus {
                        running: false,
                        endpoint: None,
                        last_error: state.last_error.clone(),
                    });
                }
            }
        }
        Ok(McpServerStatus {
            running: state.server.is_some(),
            endpoint: state
                .server
                .as_ref()
                .map(|server| server.endpoint().to_owned()),
            last_error: state.last_error.clone(),
        })
    }

    async fn stop_mcp_server(&self) {
        let mut state = self.mcp.lock().await;
        if let Some(server) = state.server.take() {
            server.stop().await;
        }
    }
}

async fn property_bool(properties: &PropertyDao, key: &str) -> Result<bool> {
    Ok(properties.get(MCP_PROPERTY_GROUP, key).await?.as_deref() == Some("true"))
}

fn bool_property(value: bool) -> String {
    value.to_string()
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
