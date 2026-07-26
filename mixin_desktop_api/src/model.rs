use std::collections::HashMap;

use mixin_desktop_core::core::device_transfer::{
    ConnectionFailedReason as CoreConnectionFailedReason,
    DeviceTransferCommand as CoreDeviceTransferCommand,
    DeviceTransferEvent as CoreDeviceTransferEvent,
};
use mixin_desktop_core::network::{ProxyConfig, ProxySettings, ProxyType};
use mixin_desktop_core::runtime::mcp::{McpServerStatus, McpSettings};
use mixin_desktop_core::runtime::model::ConversationListData;

use crate::{ClientError, ClientResult};

#[derive(Clone, Debug)]
pub struct AccountProfile {
    pub user_id: String,
    pub full_name: String,
    pub avatar_url: String,
    pub identity_number: String,
    pub biography: String,
    pub phone: String,
    pub created_at: String,
    pub is_verified: bool,
    pub fiat_currency: String,
    pub membership: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ConversationChangeEvent {
    pub conversation_ids: Vec<String>,
    pub reload_all: bool,
}

#[derive(Clone, Debug)]
pub enum ConnectionFailedReason {
    VersionNotMatched,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum DeviceTransferEvent {
    RestoreConnected,
    RestoreStart,
    RestoreSucceed,
    RestoreFailed,
    BackupServerCreated,
    BackupStart,
    BackupSucceed,
    BackupFailed,
    RestoreProgress(f64),
    BackupProgress(f64),
    RestoreNetworkSpeed(f64),
    BackupNetworkSpeed(f64),
    BackupRequestReceived,
    RestoreRequestReceived,
    ConnectionFailed(ConnectionFailedReason),
}

#[derive(Clone, Debug)]
pub enum DeviceTransferCommand {
    PullToRemote,
    PushToRemote,
    CancelRestore,
    CancelBackup,
    CancelBackupRequest,
    CancelRestoreRequest,
    ConfirmRestore,
    ConfirmBackup,
}

impl From<DeviceTransferCommand> for CoreDeviceTransferCommand {
    fn from(value: DeviceTransferCommand) -> Self {
        match value {
            DeviceTransferCommand::PullToRemote => Self::PullToRemote,
            DeviceTransferCommand::PushToRemote => Self::PushToRemote,
            DeviceTransferCommand::CancelRestore => Self::CancelRestore,
            DeviceTransferCommand::CancelBackup => Self::CancelBackup,
            DeviceTransferCommand::CancelBackupRequest => Self::CancelBackupRequest,
            DeviceTransferCommand::CancelRestoreRequest => Self::CancelRestoreRequest,
            DeviceTransferCommand::ConfirmRestore => Self::ConfirmRestore,
            DeviceTransferCommand::ConfirmBackup => Self::ConfirmBackup,
        }
    }
}

impl From<CoreDeviceTransferEvent> for DeviceTransferEvent {
    fn from(value: CoreDeviceTransferEvent) -> Self {
        match value {
            CoreDeviceTransferEvent::RestoreConnected => Self::RestoreConnected,
            CoreDeviceTransferEvent::RestoreStart => Self::RestoreStart,
            CoreDeviceTransferEvent::RestoreSucceed => Self::RestoreSucceed,
            CoreDeviceTransferEvent::RestoreFailed => Self::RestoreFailed,
            CoreDeviceTransferEvent::BackupServerCreated => Self::BackupServerCreated,
            CoreDeviceTransferEvent::BackupStart => Self::BackupStart,
            CoreDeviceTransferEvent::BackupSucceed => Self::BackupSucceed,
            CoreDeviceTransferEvent::BackupFailed => Self::BackupFailed,
            CoreDeviceTransferEvent::RestoreProgress(value) => Self::RestoreProgress(value),
            CoreDeviceTransferEvent::BackupProgress(value) => Self::BackupProgress(value),
            CoreDeviceTransferEvent::RestoreNetworkSpeed(value) => Self::RestoreNetworkSpeed(value),
            CoreDeviceTransferEvent::BackupNetworkSpeed(value) => Self::BackupNetworkSpeed(value),
            CoreDeviceTransferEvent::BackupRequestReceived => Self::BackupRequestReceived,
            CoreDeviceTransferEvent::RestoreRequestReceived => Self::RestoreRequestReceived,
            CoreDeviceTransferEvent::ConnectionFailed(reason) => {
                Self::ConnectionFailed(match reason {
                    CoreConnectionFailedReason::VersionNotMatched => {
                        ConnectionFailedReason::VersionNotMatched
                    }
                    CoreConnectionFailedReason::Unknown => ConnectionFailedReason::Unknown,
                })
            }
        }
    }
}

impl From<&sdk::Account> for AccountProfile {
    fn from(account: &sdk::Account) -> Self {
        Self {
            user_id: account.user_id.clone(),
            full_name: account.full_name.clone().unwrap_or_default(),
            avatar_url: account.avatar_url.clone().unwrap_or_default(),
            identity_number: account.identity_number.clone(),
            biography: account.biography.clone(),
            phone: account.phone.clone(),
            created_at: account.created_at.clone(),
            is_verified: account.is_verified,
            fiat_currency: account.fiat_currency.clone(),
            membership: account
                .membership
                .as_ref()
                .and_then(|membership| serde_json::to_string(membership).ok()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConversationListItem {
    pub conversation_id: String,
    pub name: String,
    pub icon_url: String,
    pub last_message: String,
    pub unseen_count: i64,
    pub mention_count: i64,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub updated_at_millis: i64,
}

impl From<ConversationListData> for ConversationListItem {
    fn from(value: ConversationListData) -> Self {
        Self {
            conversation_id: value.conversation_id,
            name: value.name,
            icon_url: value.avatar_url,
            last_message: value.last_message,
            unseen_count: value.unseen_count,
            mention_count: value.mention_count,
            is_pinned: value.is_pinned,
            is_muted: value.is_muted,
            updated_at_millis: value.updated_at_millis,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProxyItem {
    pub id: String,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProxySettingsItem {
    pub enabled: bool,
    pub selected_proxy_id: Option<String>,
    pub proxies: Vec<ProxyItem>,
}

#[derive(Clone, Debug)]
pub struct HttpResponseItem {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct McpSettingsItem {
    pub enabled: bool,
    pub token: String,
    pub draft_tools_enabled: bool,
    pub circle_management_enabled: bool,
}

#[derive(Clone, Debug)]
pub struct McpServerStatusItem {
    pub running: bool,
    pub endpoint: Option<String>,
    pub last_error: Option<String>,
}

impl From<McpSettings> for McpSettingsItem {
    fn from(settings: McpSettings) -> Self {
        Self {
            enabled: settings.enabled,
            token: settings.token,
            draft_tools_enabled: settings.draft_tools_enabled,
            circle_management_enabled: settings.circle_management_enabled,
        }
    }
}

impl From<McpSettingsItem> for McpSettings {
    fn from(settings: McpSettingsItem) -> Self {
        Self {
            enabled: settings.enabled,
            token: settings.token,
            draft_tools_enabled: settings.draft_tools_enabled,
            circle_management_enabled: settings.circle_management_enabled,
        }
    }
}

impl From<McpServerStatus> for McpServerStatusItem {
    fn from(status: McpServerStatus) -> Self {
        Self {
            running: status.running,
            endpoint: status.endpoint,
            last_error: status.last_error,
        }
    }
}

impl From<ProxyConfig> for ProxyItem {
    fn from(proxy: ProxyConfig) -> Self {
        Self {
            id: proxy.id,
            kind: match proxy.proxy_type {
                ProxyType::Http => "http".to_string(),
                ProxyType::Socks5 => "socks5".to_string(),
            },
            host: proxy.host,
            port: proxy.port,
            username: proxy.username,
            password: proxy.password,
        }
    }
}

impl TryFrom<ProxyItem> for ProxyConfig {
    type Error = ClientError;

    fn try_from(proxy: ProxyItem) -> ClientResult<Self> {
        let proxy_type = match proxy.kind.to_lowercase().as_str() {
            "http" => ProxyType::Http,
            "socks5" => ProxyType::Socks5,
            _ => {
                return Err(ClientError::InvalidArgument(
                    "unsupported proxy type".to_string(),
                ))
            }
        };
        Ok(Self {
            id: proxy.id,
            proxy_type,
            host: proxy.host,
            port: proxy.port,
            username: proxy.username,
            password: proxy.password,
        })
    }
}

impl From<ProxySettings> for ProxySettingsItem {
    fn from(settings: ProxySettings) -> Self {
        Self {
            enabled: settings.enabled,
            selected_proxy_id: settings.selected_proxy_id,
            proxies: settings.proxies.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<ProxySettingsItem> for ProxySettings {
    type Error = ClientError;

    fn try_from(settings: ProxySettingsItem) -> ClientResult<Self> {
        Ok(Self {
            enabled: settings.enabled,
            selected_proxy_id: settings.selected_proxy_id,
            proxies: settings
                .proxies
                .into_iter()
                .map(TryInto::try_into)
                .collect::<ClientResult<Vec<_>>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ProxyItem;
    use crate::ClientError;

    #[test]
    fn unsupported_proxy_kind_returns_invalid_argument() {
        let result = mixin_desktop_core::network::ProxyConfig::try_from(ProxyItem {
            id: "proxy-id".to_string(),
            kind: "ftp".to_string(),
            host: "127.0.0.1".to_string(),
            port: 21,
            username: None,
            password: None,
        });

        assert!(matches!(
            result,
            Err(ClientError::InvalidArgument(message))
                if message == "unsupported proxy type"
        ));
    }
}
