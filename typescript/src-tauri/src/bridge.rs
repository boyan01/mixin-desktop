use mixin_desktop_api::{
    AccountProfile, CircleItem, ClientError, ConversationChangeEvent, ConversationListData,
    ConversationParticipantItem, ConversationUnseenCount, DeviceTransferEvent, MessageListView,
    NotificationEvent, StickerAlbumItem, StickerDetailItem, StickerItem,
};
use serde::{Deserialize, Serialize};

macro_rules! bridge_event {
    ($name:ident, $event_name:literal, $payload:ty) => {
        #[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
        #[tauri_specta(event_name = $event_name)]
        pub struct $name(pub $payload);
    };
}

bridge_event!(AppMenuActionEvent, "app://menu-action", String);
bridge_event!(AppProtocolUrlEvent, "app://protocol-url", String);
bridge_event!(
    AccountConversationChangeEvent,
    "account://conversation-change",
    ConversationChangeEvent
);
bridge_event!(AccountMessageChangeEvent, "account://message-change", u64);
bridge_event!(
    AccountCircleChangeEvent,
    "account://circle-change",
    Vec<CircleItem>
);
bridge_event!(
    AccountUnseenCountChangeEvent,
    "account://unseen-count-change",
    Vec<ConversationUnseenCount>
);
bridge_event!(
    AccountProfileChangeEvent,
    "account://profile-change",
    AccountProfile
);
bridge_event!(
    AccountConnectionChangeEvent,
    "account://connection-change",
    bool
);
bridge_event!(AccountHealthChangeEvent, "account://health-change", String);
bridge_event!(
    AccountDeviceTransferEvent,
    "account://device-transfer",
    DeviceTransferEventDto
);
bridge_event!(
    AccountNotificationEvent,
    "account://notification",
    NotificationEvent
);

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub kind: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "DeviceTransferEvent")]
pub struct DeviceTransferEventDto {
    pub kind: String,
    pub value: Option<f64>,
    pub reason: Option<String>,
}

impl From<DeviceTransferEvent> for DeviceTransferEventDto {
    fn from(value: DeviceTransferEvent) -> Self {
        let (kind, value, reason) = match value {
            DeviceTransferEvent::RestoreConnected => ("restoreConnected", None, None),
            DeviceTransferEvent::RestoreStart => ("restoreStart", None, None),
            DeviceTransferEvent::RestoreSucceed => ("restoreSucceed", None, None),
            DeviceTransferEvent::RestoreFailed => ("restoreFailed", None, None),
            DeviceTransferEvent::BackupServerCreated => ("backupServerCreated", None, None),
            DeviceTransferEvent::BackupStart => ("backupStart", None, None),
            DeviceTransferEvent::BackupSucceed => ("backupSucceed", None, None),
            DeviceTransferEvent::BackupFailed => ("backupFailed", None, None),
            DeviceTransferEvent::RestoreProgress(value) => ("restoreProgress", Some(value), None),
            DeviceTransferEvent::BackupProgress(value) => ("backupProgress", Some(value), None),
            DeviceTransferEvent::RestoreNetworkSpeed(value) => {
                ("restoreNetworkSpeed", Some(value), None)
            }
            DeviceTransferEvent::BackupNetworkSpeed(value) => {
                ("backupNetworkSpeed", Some(value), None)
            }
            DeviceTransferEvent::BackupRequestReceived => ("backupRequestReceived", None, None),
            DeviceTransferEvent::RestoreRequestReceived => ("restoreRequestReceived", None, None),
            DeviceTransferEvent::ConnectionFailed(reason) => (
                "connectionFailed",
                None,
                Some(match reason {
                    mixin_desktop_api::ConnectionFailedReason::VersionNotMatched => {
                        "versionNotMatched".to_string()
                    }
                    mixin_desktop_api::ConnectionFailedReason::Unknown => "unknown".to_string(),
                }),
            ),
        };
        Self {
            kind: kind.to_string(),
            value,
            reason,
        }
    }
}

impl From<ClientError> for CommandError {
    fn from(error: ClientError) -> Self {
        let kind = match &error {
            ClientError::Unauthorized => "unauthorized",
            ClientError::NotFound => "notFound",
            ClientError::Cancelled => "cancelled",
            ClientError::InvalidArgument(_) => "invalidArgument",
            ClientError::Internal(_) => "internal",
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "Message")]
pub struct MessageDto {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_identity_number: Option<String>,
    pub sender_avatar_url: String,
    pub sender_is_verified: bool,
    pub sender_membership: Option<String>,
    pub sender_relationship: String,
    pub sender_app_id: Option<String>,
    pub sender_is_bot: bool,
    pub sender_participant_id: Option<String>,
    pub sender_role: Option<String>,
    pub category: String,
    pub content: String,
    pub status: String,
    pub created_at_micros: i64,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_duration: String,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub media_status: String,
    pub media_waveform: Option<String>,
    pub thumb_image: Option<String>,
    pub thumb_url: Option<String>,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub quote_user_membership: Option<String>,
    pub caption: Option<String>,
    pub action: Option<String>,
    pub participant_id: Option<String>,
    pub participant_full_name: Option<String>,
    pub media_name: Option<String>,
    pub sticker_id: Option<String>,
    pub sticker_asset_url: Option<String>,
    pub sticker_asset_type: Option<String>,
    pub shared_user_id: Option<String>,
    pub shared_user_full_name: Option<String>,
    pub shared_user_identity_number: Option<String>,
    pub shared_user_avatar_url: Option<String>,
    pub shared_user_is_verified: bool,
    pub shared_user_membership: Option<String>,
    pub shared_user_app_id: Option<String>,
    pub mention_read: Option<bool>,
    pub pinned: bool,
    pub expire_in: Option<i64>,
    pub snapshot_amount: Option<String>,
    pub snapshot_id: Option<String>,
    pub snapshot_asset_symbol: Option<String>,
    pub snapshot_asset_icon_url: Option<String>,
    pub snapshot_chain_icon_url: Option<String>,
    pub snapshot_memo: Option<String>,
    pub snapshot_type: Option<String>,
    pub snapshot_opponent_id: Option<String>,
    pub snapshot_transaction_hash: Option<String>,
    pub snapshot_created_at: Option<String>,
    pub inscription_hash: Option<String>,
    pub inscription_sequence: Option<i64>,
    pub inscription_content_type: Option<String>,
    pub inscription_content_url: Option<String>,
    pub inscription_name: Option<String>,
    pub inscription_icon_url: Option<String>,
    pub conversation_owner_id: Option<String>,
    pub conversation_category: Option<String>,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SendAttachmentRequest {
    pub conversation_id: String,
    pub path: String,
    pub kind: String,
    pub mime_type: String,
    pub name: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_millis: Option<i64>,
    pub thumbnail: Option<String>,
    pub caption: Option<String>,
    pub quote_message_id: Option<String>,
    pub silent: bool,
}

#[derive(Clone, Debug, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SendRemoteImageRequest {
    pub conversation_id: String,
    pub url: String,
    pub preview_url: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub mime_type: String,
    pub silent: bool,
}

impl From<MessageListView> for MessageDto {
    fn from(value: MessageListView) -> Self {
        Self {
            message_id: value.message_id,
            conversation_id: value.conversation_id,
            sender_id: value.sender_id,
            sender_name: value.sender_name,
            sender_identity_number: value.sender_identity_number,
            sender_avatar_url: value.sender_avatar_url,
            sender_is_verified: value.sender_is_verified,
            sender_membership: value.sender_membership,
            sender_relationship: value.sender_relationship,
            sender_app_id: value.sender_app_id,
            sender_is_bot: value.sender_is_bot,
            sender_participant_id: value.sender_participant_id,
            sender_role: value.sender_role,
            category: value.category,
            content: value.content,
            status: value.status,
            created_at_micros: value.created_at_micros,
            media_url: value.media_url,
            media_mime_type: value.media_mime_type,
            media_size: value.media_size,
            media_duration: value.media_duration,
            media_width: value.media_width,
            media_height: value.media_height,
            media_status: value.media_status,
            media_waveform: value.media_waveform,
            thumb_image: value.thumb_image,
            thumb_url: value.thumb_url,
            quote_message_id: value.quote_message_id,
            quote_content: value.quote_content,
            quote_user_membership: value.quote_user_membership,
            caption: value.caption,
            action: value.action,
            participant_id: value.participant_id,
            participant_full_name: value.participant_full_name,
            media_name: value.media_name,
            sticker_id: value.sticker_id,
            sticker_asset_url: value.sticker_asset_url,
            sticker_asset_type: value.sticker_asset_type,
            shared_user_id: value.shared_user_id,
            shared_user_full_name: value.shared_user_full_name,
            shared_user_identity_number: value.shared_user_identity_number,
            shared_user_avatar_url: value.shared_user_avatar_url,
            shared_user_is_verified: value.shared_user_is_verified,
            shared_user_membership: value.shared_user_membership,
            shared_user_app_id: value.shared_user_app_id,
            mention_read: value.mention_read,
            pinned: value.pinned,
            expire_in: value.expire_in,
            snapshot_amount: value.snapshot_amount,
            snapshot_id: value.snapshot_id,
            snapshot_asset_symbol: value.snapshot_asset_symbol,
            snapshot_asset_icon_url: value.snapshot_asset_icon_url,
            snapshot_chain_icon_url: value.snapshot_chain_icon_url,
            snapshot_memo: value.snapshot_memo,
            snapshot_type: value.snapshot_type,
            snapshot_opponent_id: value.snapshot_opponent_id,
            snapshot_transaction_hash: value.snapshot_transaction_hash,
            snapshot_created_at: value.snapshot_created_at,
            inscription_hash: value.inscription_hash,
            inscription_sequence: value.inscription_sequence,
            inscription_content_type: value.inscription_content_type,
            inscription_content_url: value.inscription_content_url,
            inscription_name: value.inscription_name,
            inscription_icon_url: value.inscription_icon_url,
            conversation_owner_id: value.conversation_owner_id,
            conversation_category: value.conversation_category,
        }
    }
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "BootstrapData")]
pub struct BootstrapDto {
    pub signed_in: bool,
    pub profile: Option<AccountProfile>,
    pub conversations: Vec<ConversationListData>,
    pub account_health: Option<String>,
    pub connection_status: Option<bool>,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "LoginSession")]
pub struct LoginSessionDto {
    pub request_id: u64,
    pub auth_url: String,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "DownloadSettings")]
pub struct DownloadSettingsDto {
    pub photo: bool,
    pub video: bool,
    pub file: bool,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "ConversationParticipant")]
pub struct ConversationParticipantDto {
    pub user_id: String,
    pub role: Option<String>,
    pub identity_number: String,
    pub full_name: String,
    pub avatar_url: String,
    pub biography: String,
    pub is_verified: bool,
    pub is_bot: bool,
    pub membership: Option<String>,
}

impl From<ConversationParticipantItem> for ConversationParticipantDto {
    fn from(value: ConversationParticipantItem) -> Self {
        Self {
            user_id: value.user_id,
            role: value.role,
            identity_number: value.identity_number,
            full_name: value.full_name,
            avatar_url: value.avatar_url,
            biography: value.biography,
            is_verified: value.is_verified,
            is_bot: value.is_bot,
            membership: value.membership,
        }
    }
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "Sticker")]
pub struct StickerDto {
    pub sticker_id: String,
    pub album_id: Option<String>,
    pub name: String,
    pub asset_url: String,
    pub asset_width: i32,
    pub asset_height: i32,
    pub asset_type: String,
}

#[derive(Clone, Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
#[serde(rename = "StickerDetail")]
pub struct StickerDetailDto {
    pub sticker: StickerDto,
    pub album: Option<StickerAlbumItem>,
    pub album_stickers: Vec<StickerDto>,
    pub is_personal: bool,
}

impl From<StickerItem> for StickerDto {
    fn from(value: StickerItem) -> Self {
        Self {
            sticker_id: value.sticker_id,
            album_id: value.album_id,
            name: value.name,
            asset_url: value.asset_url,
            asset_width: value.asset_width,
            asset_height: value.asset_height,
            asset_type: value.asset_type,
        }
    }
}

impl From<StickerDetailItem> for StickerDetailDto {
    fn from(value: StickerDetailItem) -> Self {
        Self {
            sticker: value.sticker.into(),
            album: value.album,
            album_stickers: value.album_stickers.into_iter().map(Into::into).collect(),
            is_personal: value.is_personal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandError, DeviceTransferEventDto};
    use mixin_desktop_api::{ClientError, ConnectionFailedReason, DeviceTransferEvent};

    #[test]
    fn lowers_public_errors_to_stable_frontend_kinds() {
        let cases = [
            (ClientError::Unauthorized, "unauthorized"),
            (ClientError::NotFound, "notFound"),
            (ClientError::Cancelled, "cancelled"),
            (
                ClientError::InvalidArgument("bad value".to_string()),
                "invalidArgument",
            ),
            (ClientError::Internal("failed".to_string()), "internal"),
        ];

        for (error, expected) in cases {
            assert_eq!(CommandError::from(error).kind, expected);
        }
    }

    #[test]
    fn lowers_device_transfer_events_to_stable_frontend_values() {
        let progress = DeviceTransferEventDto::from(DeviceTransferEvent::RestoreProgress(0.5));
        assert_eq!(progress.kind, "restoreProgress");
        assert_eq!(progress.value, Some(0.5));

        let failure = DeviceTransferEventDto::from(DeviceTransferEvent::ConnectionFailed(
            ConnectionFailedReason::VersionNotMatched,
        ));
        assert_eq!(failure.kind, "connectionFailed");
        assert_eq!(failure.reason.as_deref(), Some("versionNotMatched"));
    }
}
