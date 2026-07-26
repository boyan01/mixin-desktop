use mixin_desktop_api::{
    ConnectionFailedReason as ApiConnectionFailedReason,
    DeviceTransferCommand as ApiDeviceTransferCommand,
    DeviceTransferEvent as ApiDeviceTransferEvent,
};

#[flutter_rust_bridge::frb(non_opaque)]
pub enum ConnectionFailedReason {
    VersionNotMatched,
    Unknown,
}

#[flutter_rust_bridge::frb(non_opaque)]
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

#[flutter_rust_bridge::frb(non_opaque)]
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

impl From<DeviceTransferCommand> for ApiDeviceTransferCommand {
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

impl From<ApiDeviceTransferEvent> for DeviceTransferEvent {
    fn from(value: ApiDeviceTransferEvent) -> Self {
        match value {
            ApiDeviceTransferEvent::RestoreConnected => Self::RestoreConnected,
            ApiDeviceTransferEvent::RestoreStart => Self::RestoreStart,
            ApiDeviceTransferEvent::RestoreSucceed => Self::RestoreSucceed,
            ApiDeviceTransferEvent::RestoreFailed => Self::RestoreFailed,
            ApiDeviceTransferEvent::BackupServerCreated => Self::BackupServerCreated,
            ApiDeviceTransferEvent::BackupStart => Self::BackupStart,
            ApiDeviceTransferEvent::BackupSucceed => Self::BackupSucceed,
            ApiDeviceTransferEvent::BackupFailed => Self::BackupFailed,
            ApiDeviceTransferEvent::RestoreProgress(value) => Self::RestoreProgress(value),
            ApiDeviceTransferEvent::BackupProgress(value) => Self::BackupProgress(value),
            ApiDeviceTransferEvent::RestoreNetworkSpeed(value) => Self::RestoreNetworkSpeed(value),
            ApiDeviceTransferEvent::BackupNetworkSpeed(value) => Self::BackupNetworkSpeed(value),
            ApiDeviceTransferEvent::BackupRequestReceived => Self::BackupRequestReceived,
            ApiDeviceTransferEvent::RestoreRequestReceived => Self::RestoreRequestReceived,
            ApiDeviceTransferEvent::ConnectionFailed(reason) => {
                Self::ConnectionFailed(match reason {
                    ApiConnectionFailedReason::VersionNotMatched => {
                        ConnectionFailedReason::VersionNotMatched
                    }
                    ApiConnectionFailedReason::Unknown => ConnectionFailedReason::Unknown,
                })
            }
        }
    }
}
