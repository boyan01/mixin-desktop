use std::collections::HashMap;

use mixin_desktop_api::{
    AccountProfile, CircleItem, CodeResult, ConnectionFailedReason, ConversationChangeEvent,
    ConversationDetailItem, ConversationListData, ConversationParticipantItem,
    ConversationStorageUsage, ConversationUnseenCount, DeviceTransferCommand, DeviceTransferEvent,
    GroupAvatar, GroupConversationItem, HttpResponseItem, ImageMessageView, McpServerStatusItem,
    McpSettingsItem, MessageListView, NotificationEvent, ProxyItem, ProxySettingsItem,
    SharedAppItem, SnapshotDetailItem, StickerAlbumItem, StickerDetailItem, StickerItem,
    StorageCategoryUsage, UserProfileItem,
};

#[uniffi::remote(Record)]
pub struct HttpResponseItem {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[uniffi::remote(Record)]
pub struct ImageMessageView {
    pub message_id: String,
    pub created_at_micros: i64,
    pub media_url: String,
    pub media_name: Option<String>,
    pub thumb_image: Option<String>,
    pub can_forward: bool,
    pub user_id: String,
    pub user_full_name: String,
    pub user_identity_number: String,
    pub avatar_url: String,
}

#[uniffi::remote(Record)]
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

#[uniffi::remote(Record)]
pub struct NotificationEvent {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_name: String,
    pub category: String,
    pub content: String,
    pub created_at_micros: i64,
    pub conversation_name: String,
    pub conversation_category: String,
    pub dismiss_message_id: Option<String>,
}

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

#[uniffi::remote(Enum)]
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

#[uniffi::remote(Record)]
pub struct CircleItem {
    pub circle_id: String,
    pub name: String,
    pub conversation_count: i64,
}

#[uniffi::remote(Record)]
pub struct ConversationUnseenCount {
    pub category: String,
    pub circle_id: Option<String>,
    pub count: i64,
    pub muted_count: i64,
}

#[uniffi::remote(Record)]
pub struct ConversationChangeEvent {
    pub conversation_ids: Vec<String>,
    pub reload_all: bool,
}

#[uniffi::remote(Record)]
pub struct ConversationListData {
    pub conversation_id: String,
    pub owner_id: String,
    pub name: String,
    pub avatar_url: String,
    pub category: String,
    pub draft: String,
    pub status: i32,
    pub last_read_message_id: Option<String>,
    pub last_message: String,
    pub last_message_category: Option<String>,
    pub last_message_status: Option<String>,
    pub last_message_sender_id: Option<String>,
    pub last_message_sender_name: Option<String>,
    pub last_message_action: Option<String>,
    pub last_message_participant_id: Option<String>,
    pub last_message_participant_name: Option<String>,
    pub updated_at_millis: i64,
    pub unseen_count: i64,
    pub mention_count: i64,
    pub is_muted: bool,
    pub is_verified: bool,
    pub is_scam: bool,
    pub is_bot: bool,
    pub is_bot_group: bool,
    pub membership: Option<String>,
    pub is_pinned: bool,
    pub pin_time_millis: i64,
    pub relationship: String,
    pub identity_number: String,
    pub circle_ids: Vec<String>,
    pub participant_count: i64,
    pub group_avatars: Vec<GroupAvatar>,
}

#[uniffi::remote(Record)]
pub struct GroupConversationItem {
    pub conversation_id: String,
    pub name: String,
    pub avatar_url: String,
    pub participant_count: i64,
}

#[uniffi::remote(Record)]
pub struct GroupAvatar {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
}

#[uniffi::remote(Record)]
pub struct CodeResult {
    pub kind: String,
    pub user_id: Option<String>,
    pub conversation_id: Option<String>,
    pub conversation_name: Option<String>,
    pub participant_count: i64,
    pub participant_avatars: Vec<GroupAvatar>,
    pub already_member: bool,
    pub asset_id: Option<String>,
    pub asset_symbol: Option<String>,
    pub asset_icon_url: Option<String>,
    pub chain_icon_url: Option<String>,
    pub amount: Option<String>,
    pub senders: Vec<String>,
    pub receivers: Vec<String>,
    pub threshold: i64,
    pub state: Option<String>,
    pub action: Option<String>,
}

#[uniffi::remote(Record)]
pub struct SnapshotDetailItem {
    pub snapshot_id: String,
    pub trace_id: Option<String>,
    pub snapshot_type: String,
    pub asset_id: String,
    pub amount: String,
    pub created_at_millis: i64,
    pub opponent_id: Option<String>,
    pub opponent_name: Option<String>,
    pub transaction_hash: Option<String>,
    pub sender: Option<String>,
    pub receiver: Option<String>,
    pub memo: Option<String>,
    pub confirmations: Option<i32>,
    pub snapshot_hash: Option<String>,
    pub opening_balance: Option<String>,
    pub closing_balance: Option<String>,
    pub symbol: String,
    pub asset_name: String,
    pub asset_icon_url: String,
    pub chain_icon_url: String,
    pub asset_confirmations: i64,
    pub asset_tag: Option<String>,
    pub current_user_name: String,
    pub is_safe: bool,
    pub price_usd: Option<String>,
    pub fiat_rate: Option<f64>,
    pub ticker_price_usd: Option<String>,
    pub deposit_hash: Option<String>,
    pub withdrawal_hash: Option<String>,
    pub withdrawal_receiver: Option<String>,
}

#[derive(uniffi::Record)]
pub struct MessageItem {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_identity_number: Option<String>,
    pub sender_avatar_url: String,
    pub sender_relationship: String,
    pub sender_app_id: Option<String>,
    pub sender_is_bot: bool,
    pub sender_is_verified: bool,
    pub sender_membership: Option<String>,
    pub sender_participant_id: Option<String>,
    pub sender_role: Option<String>,
    pub conversation_owner_id: Option<String>,
    pub conversation_category: Option<String>,
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
    pub thumb_image: Option<String>,
    pub thumb_url: Option<String>,
    pub media_status: String,
    pub media_name: Option<String>,
    pub media_waveform: Option<String>,
    pub caption: Option<String>,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub action: Option<String>,
    pub participant_full_name: Option<String>,
    pub snapshot_id: Option<String>,
    pub snapshot_type: Option<String>,
    pub snapshot_amount: Option<String>,
    pub snapshot_memo: Option<String>,
    pub snapshot_asset_symbol: Option<String>,
    pub snapshot_asset_icon_url: Option<String>,
    pub snapshot_chain_icon_url: Option<String>,
    pub snapshot_opponent_id: Option<String>,
    pub snapshot_transaction_hash: Option<String>,
    pub snapshot_created_at: Option<String>,
    pub inscription_hash: Option<String>,
    pub inscription_collection_hash: Option<String>,
    pub inscription_sequence: Option<i64>,
    pub inscription_content_type: Option<String>,
    pub inscription_content_url: Option<String>,
    pub inscription_name: Option<String>,
    pub inscription_icon_url: Option<String>,
    pub shared_user_full_name: Option<String>,
    pub shared_user_identity_number: Option<String>,
    pub shared_user_id: Option<String>,
    pub shared_user_avatar_url: Option<String>,
    pub shared_user_is_verified: bool,
    pub shared_user_membership: Option<String>,
    pub shared_user_app_id: Option<String>,
    pub sticker_id: Option<String>,
    pub sticker_asset_url: Option<String>,
    pub sticker_asset_width: Option<i32>,
    pub sticker_asset_height: Option<i32>,
    pub sticker_asset_name: Option<String>,
    pub sticker_asset_type: Option<String>,
    pub hyperlink: Option<String>,
    pub pinned: bool,
    pub expire_in: Option<i64>,
}

impl From<MessageListView> for MessageItem {
    fn from(value: MessageListView) -> Self {
        Self {
            message_id: value.message_id,
            conversation_id: value.conversation_id,
            sender_id: value.sender_id,
            sender_name: value.sender_name,
            sender_identity_number: value.sender_identity_number,
            sender_avatar_url: value.sender_avatar_url,
            sender_relationship: value.sender_relationship,
            sender_app_id: value.sender_app_id,
            sender_is_bot: value.sender_is_bot,
            sender_is_verified: value.sender_is_verified,
            sender_membership: value.sender_membership,
            sender_participant_id: value.sender_participant_id,
            sender_role: value.sender_role,
            conversation_owner_id: value.conversation_owner_id,
            conversation_category: value.conversation_category,
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
            thumb_image: value.thumb_image,
            thumb_url: value.thumb_url,
            media_status: value.media_status,
            media_name: value.media_name,
            media_waveform: value.media_waveform,
            caption: value.caption,
            quote_message_id: value.quote_message_id,
            quote_content: value.quote_content,
            action: value.action,
            participant_full_name: value.participant_full_name,
            snapshot_id: value.snapshot_id,
            snapshot_type: value.snapshot_type,
            snapshot_amount: value.snapshot_amount,
            snapshot_memo: value.snapshot_memo,
            snapshot_asset_symbol: value.snapshot_asset_symbol,
            snapshot_asset_icon_url: value.snapshot_asset_icon_url,
            snapshot_chain_icon_url: value.snapshot_chain_icon_url,
            snapshot_opponent_id: value.snapshot_opponent_id,
            snapshot_transaction_hash: value.snapshot_transaction_hash,
            snapshot_created_at: value.snapshot_created_at,
            inscription_hash: value.inscription_hash,
            inscription_collection_hash: value.inscription_collection_hash,
            inscription_sequence: value.inscription_sequence,
            inscription_content_type: value.inscription_content_type,
            inscription_content_url: value.inscription_content_url,
            inscription_name: value.inscription_name,
            inscription_icon_url: value.inscription_icon_url,
            shared_user_full_name: value.shared_user_full_name,
            shared_user_identity_number: value.shared_user_identity_number,
            shared_user_id: value.shared_user_id,
            shared_user_avatar_url: value.shared_user_avatar_url,
            shared_user_is_verified: value.shared_user_is_verified,
            shared_user_membership: value.shared_user_membership,
            shared_user_app_id: value.shared_user_app_id,
            sticker_id: value.sticker_id,
            sticker_asset_url: value.sticker_asset_url,
            sticker_asset_width: value.sticker_asset_width,
            sticker_asset_height: value.sticker_asset_height,
            sticker_asset_name: value.sticker_asset_name,
            sticker_asset_type: value.sticker_asset_type,
            hyperlink: value.hyperlink,
            pinned: value.pinned,
            expire_in: value.expire_in,
        }
    }
}

#[uniffi::remote(Record)]
pub struct SharedAppItem {
    pub app_id: String,
    pub name: String,
    pub icon_url: String,
    pub description: String,
    pub home_uri: String,
}

#[uniffi::remote(Record)]
pub struct StickerItem {
    pub sticker_id: String,
    pub album_id: Option<String>,
    pub name: String,
    pub asset_url: String,
    pub asset_width: i32,
    pub asset_height: i32,
    pub asset_type: String,
    pub created_at_millis: i64,
    pub last_use_at_millis: Option<i64>,
}

#[uniffi::remote(Record)]
pub struct StickerAlbumItem {
    pub album_id: String,
    pub name: String,
    pub icon_url: String,
    pub category: String,
    pub description: String,
    pub banner: Option<String>,
    pub added: bool,
    pub is_verified: bool,
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

#[uniffi::remote(Record)]
pub struct StickerDetailItem {
    pub sticker: StickerItem,
    pub album: Option<StickerAlbumItem>,
    pub album_stickers: Vec<StickerItem>,
    pub is_personal: bool,
}

#[uniffi::remote(Record)]
pub struct UserProfileItem {
    pub user_id: String,
    pub identity_number: String,
    pub full_name: String,
    pub avatar_url: String,
    pub biography: String,
    pub is_verified: bool,
    pub is_bot: bool,
    pub relationship: String,
    pub code_url: String,
    pub membership: Option<String>,
}

#[uniffi::remote(Record)]
pub struct ConversationDetailItem {
    pub conversation_id: String,
    pub name: String,
    pub announcement: String,
    pub code_url: String,
    pub created_at_millis: i64,
    pub mute_until_millis: i64,
    pub expire_in: i64,
}

#[uniffi::remote(Record)]
pub struct ConversationParticipantItem {
    pub user_id: String,
    pub role: Option<String>,
    pub created_at_millis: i64,
    pub identity_number: String,
    pub full_name: String,
    pub avatar_url: String,
    pub biography: String,
    pub is_verified: bool,
    pub is_bot: bool,
    pub relationship: String,
    pub membership: Option<String>,
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

#[uniffi::remote(Record)]
pub struct ConversationStorageUsage {
    pub conversation: ConversationListData,
    pub size_bytes: i64,
}

#[uniffi::remote(Record)]
pub struct StorageCategoryUsage {
    pub category: String,
    pub size_bytes: i64,
}

#[uniffi::remote(Record)]
pub struct ProxyItem {
    pub id: String,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[uniffi::remote(Record)]
pub struct ProxySettingsItem {
    pub enabled: bool,
    pub selected_proxy_id: Option<String>,
    pub proxies: Vec<ProxyItem>,
}

#[uniffi::remote(Record)]
pub struct McpSettingsItem {
    pub enabled: bool,
    pub token: String,
    pub draft_tools_enabled: bool,
    pub circle_management_enabled: bool,
}

#[uniffi::remote(Record)]
pub struct McpServerStatusItem {
    pub running: bool,
    pub endpoint: Option<String>,
    pub last_error: Option<String>,
}
