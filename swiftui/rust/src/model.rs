use mixin_desktop_api::{ConnectionFailedReason, DeviceTransferEvent};

#[derive(uniffi::Enum)]
pub enum ConnectionFailedReasonItem {
    VersionNotMatched,
    Unknown,
}

#[derive(uniffi::Enum)]
pub enum DeviceTransferEventItem {
    RestoreConnected,
    RestoreStart,
    RestoreSucceed,
    RestoreFailed,
    BackupServerCreated,
    BackupStart,
    BackupSucceed,
    BackupFailed,
    RestoreProgress { value: f64 },
    BackupProgress { value: f64 },
    RestoreNetworkSpeed { bytes_per_second: f64 },
    BackupNetworkSpeed { bytes_per_second: f64 },
    BackupRequestReceived,
    RestoreRequestReceived,
    ConnectionFailed { reason: ConnectionFailedReasonItem },
}

impl From<DeviceTransferEvent> for DeviceTransferEventItem {
    fn from(value: DeviceTransferEvent) -> Self {
        match value {
            DeviceTransferEvent::RestoreConnected => Self::RestoreConnected,
            DeviceTransferEvent::RestoreStart => Self::RestoreStart,
            DeviceTransferEvent::RestoreSucceed => Self::RestoreSucceed,
            DeviceTransferEvent::RestoreFailed => Self::RestoreFailed,
            DeviceTransferEvent::BackupServerCreated => Self::BackupServerCreated,
            DeviceTransferEvent::BackupStart => Self::BackupStart,
            DeviceTransferEvent::BackupSucceed => Self::BackupSucceed,
            DeviceTransferEvent::BackupFailed => Self::BackupFailed,
            DeviceTransferEvent::RestoreProgress(value) => Self::RestoreProgress { value },
            DeviceTransferEvent::BackupProgress(value) => Self::BackupProgress { value },
            DeviceTransferEvent::RestoreNetworkSpeed(bytes_per_second) => {
                Self::RestoreNetworkSpeed { bytes_per_second }
            }
            DeviceTransferEvent::BackupNetworkSpeed(bytes_per_second) => {
                Self::BackupNetworkSpeed { bytes_per_second }
            }
            DeviceTransferEvent::BackupRequestReceived => Self::BackupRequestReceived,
            DeviceTransferEvent::RestoreRequestReceived => Self::RestoreRequestReceived,
            DeviceTransferEvent::ConnectionFailed(reason) => Self::ConnectionFailed {
                reason: match reason {
                    ConnectionFailedReason::VersionNotMatched => {
                        ConnectionFailedReasonItem::VersionNotMatched
                    }
                    ConnectionFailedReason::Unknown => ConnectionFailedReasonItem::Unknown,
                },
            },
        }
    }
}

#[cfg(test)]
mod device_transfer_tests {
    use mixin_desktop_api::{ConnectionFailedReason, DeviceTransferEvent};

    use super::{ConnectionFailedReasonItem, DeviceTransferEventItem};

    #[test]
    fn device_transfer_events_preserve_payloads_and_failure_reason() {
        assert!(matches!(
            DeviceTransferEventItem::from(DeviceTransferEvent::RestoreProgress(42.5)),
            DeviceTransferEventItem::RestoreProgress { value } if value == 42.5
        ));
        assert!(matches!(
            DeviceTransferEventItem::from(DeviceTransferEvent::BackupNetworkSpeed(2048.0)),
            DeviceTransferEventItem::BackupNetworkSpeed { bytes_per_second }
                if bytes_per_second == 2048.0
        ));
        assert!(matches!(
            DeviceTransferEventItem::from(DeviceTransferEvent::ConnectionFailed(
                ConnectionFailedReason::VersionNotMatched
            )),
            DeviceTransferEventItem::ConnectionFailed {
                reason: ConnectionFailedReasonItem::VersionNotMatched
            }
        ));
    }
}

#[derive(uniffi::Record)]
pub struct StickerAlbumSection {
    pub album: StickerAlbumItem,
    pub stickers: Vec<StickerItem>,
}

#[derive(uniffi::Record)]
pub struct StickerLibrary {
    pub recent: Vec<StickerItem>,
    pub personal: Vec<StickerItem>,
    pub albums: Vec<StickerAlbumSection>,
}

#[derive(uniffi::Enum)]
pub enum ParticipantAction {
    Add,
    Remove,
    MakeAdmin,
    DismissAdmin,
}

impl ParticipantAction {
    pub(crate) fn api_update(&self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Add => ("ADD", None),
            Self::Remove => ("REMOVE", None),
            Self::MakeAdmin => ("ROLE", Some("ADMIN")),
            Self::DismissAdmin => ("ROLE", None),
        }
    }
}

#[cfg(test)]
mod participant_action_tests {
    use super::ParticipantAction;

    #[test]
    fn participant_actions_map_to_supported_api_updates() {
        let cases = [
            (ParticipantAction::Add, "ADD", None),
            (ParticipantAction::Remove, "REMOVE", None),
            (ParticipantAction::MakeAdmin, "ROLE", Some("ADMIN")),
            (ParticipantAction::DismissAdmin, "ROLE", None),
        ];

        for (action, expected_action, expected_role) in cases {
            let (api_action, role) = action.api_update();
            assert_eq!(api_action, expected_action);
            assert_eq!(role, expected_role);
        }
    }
}

include!("generated/model.rs");
