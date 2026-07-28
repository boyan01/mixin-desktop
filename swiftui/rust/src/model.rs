use std::collections::HashMap;

use mixin_desktop_api::{
    AccountProfile, CircleItem, CodeResult, ConnectionFailedReason, ConversationChangeEvent,
    ConversationDetailItem, ConversationListData, ConversationParticipantItem,
    ConversationStorageUsage, ConversationUnseenCount, DeviceTransferCommand, DeviceTransferEvent,
    GroupAvatar, ImageMessageView, McpServerStatusItem, McpSettingsItem, MessageListView,
    NotificationEvent, ProxyItem, ProxySettingsItem, SharedAppItem, SnapshotDetailItem,
    StickerAlbumItem, StickerDetailItem, StickerItem, StorageCategoryUsage, UserProfileItem,
};

#[derive(uniffi::Record)]
pub struct SwiftHttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl From<mixin_desktop_api::HttpResponseItem> for SwiftHttpResponse {
    fn from(value: mixin_desktop_api::HttpResponseItem) -> Self {
        Self {
            status_code: value.status_code,
            headers: value.headers,
            body: value.body,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftImageMessageItem {
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

impl From<ImageMessageView> for SwiftImageMessageItem {
    fn from(value: ImageMessageView) -> Self {
        Self {
            message_id: value.message_id,
            created_at_micros: value.created_at_micros,
            media_url: value.media_url,
            media_name: value.media_name,
            thumb_image: value.thumb_image,
            can_forward: value.can_forward,
            user_id: value.user_id,
            user_full_name: value.user_full_name,
            user_identity_number: value.user_identity_number,
            avatar_url: value.avatar_url,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftAccountProfile {
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

#[derive(uniffi::Record)]
pub struct SwiftNotificationEvent {
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

impl From<NotificationEvent> for SwiftNotificationEvent {
    fn from(value: NotificationEvent) -> Self {
        Self {
            message_id: value.message_id,
            conversation_id: value.conversation_id,
            sender_name: value.sender_name,
            category: value.category,
            content: value.content,
            created_at_micros: value.created_at_micros,
            conversation_name: value.conversation_name,
            conversation_category: value.conversation_category,
            dismiss_message_id: value.dismiss_message_id,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum SwiftConnectionFailedReason {
    VersionNotMatched,
    Unknown,
}

#[derive(uniffi::Enum)]
pub enum SwiftDeviceTransferEvent {
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
    ConnectionFailed { reason: SwiftConnectionFailedReason },
}

impl From<DeviceTransferEvent> for SwiftDeviceTransferEvent {
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
                        SwiftConnectionFailedReason::VersionNotMatched
                    }
                    ConnectionFailedReason::Unknown => SwiftConnectionFailedReason::Unknown,
                },
            },
        }
    }
}

#[derive(uniffi::Enum)]
pub enum SwiftDeviceTransferCommand {
    PullToRemote,
    PushToRemote,
    CancelRestore,
    CancelBackup,
    CancelBackupRequest,
    CancelRestoreRequest,
    ConfirmRestore,
    ConfirmBackup,
}

impl From<SwiftDeviceTransferCommand> for DeviceTransferCommand {
    fn from(value: SwiftDeviceTransferCommand) -> Self {
        match value {
            SwiftDeviceTransferCommand::PullToRemote => Self::PullToRemote,
            SwiftDeviceTransferCommand::PushToRemote => Self::PushToRemote,
            SwiftDeviceTransferCommand::CancelRestore => Self::CancelRestore,
            SwiftDeviceTransferCommand::CancelBackup => Self::CancelBackup,
            SwiftDeviceTransferCommand::CancelBackupRequest => Self::CancelBackupRequest,
            SwiftDeviceTransferCommand::CancelRestoreRequest => Self::CancelRestoreRequest,
            SwiftDeviceTransferCommand::ConfirmRestore => Self::ConfirmRestore,
            SwiftDeviceTransferCommand::ConfirmBackup => Self::ConfirmBackup,
        }
    }
}

#[cfg(test)]
mod device_transfer_tests {
    use mixin_desktop_api::{ConnectionFailedReason, DeviceTransferCommand, DeviceTransferEvent};

    use super::{
        SwiftConnectionFailedReason, SwiftDeviceTransferCommand, SwiftDeviceTransferEvent,
    };

    #[test]
    fn device_transfer_events_preserve_payloads_and_failure_reason() {
        assert!(matches!(
            SwiftDeviceTransferEvent::from(DeviceTransferEvent::RestoreProgress(42.5)),
            SwiftDeviceTransferEvent::RestoreProgress { value } if value == 42.5
        ));
        assert!(matches!(
            SwiftDeviceTransferEvent::from(DeviceTransferEvent::BackupNetworkSpeed(2048.0)),
            SwiftDeviceTransferEvent::BackupNetworkSpeed { bytes_per_second }
                if bytes_per_second == 2048.0
        ));
        assert!(matches!(
            SwiftDeviceTransferEvent::from(DeviceTransferEvent::ConnectionFailed(
                ConnectionFailedReason::VersionNotMatched
            )),
            SwiftDeviceTransferEvent::ConnectionFailed {
                reason: SwiftConnectionFailedReason::VersionNotMatched
            }
        ));
    }

    #[test]
    fn device_transfer_commands_map_to_the_public_account_api() {
        let cases = [
            (
                SwiftDeviceTransferCommand::PullToRemote,
                DeviceTransferCommand::PullToRemote,
            ),
            (
                SwiftDeviceTransferCommand::PushToRemote,
                DeviceTransferCommand::PushToRemote,
            ),
            (
                SwiftDeviceTransferCommand::CancelRestore,
                DeviceTransferCommand::CancelRestore,
            ),
            (
                SwiftDeviceTransferCommand::CancelBackup,
                DeviceTransferCommand::CancelBackup,
            ),
            (
                SwiftDeviceTransferCommand::CancelBackupRequest,
                DeviceTransferCommand::CancelBackupRequest,
            ),
            (
                SwiftDeviceTransferCommand::CancelRestoreRequest,
                DeviceTransferCommand::CancelRestoreRequest,
            ),
            (
                SwiftDeviceTransferCommand::ConfirmRestore,
                DeviceTransferCommand::ConfirmRestore,
            ),
            (
                SwiftDeviceTransferCommand::ConfirmBackup,
                DeviceTransferCommand::ConfirmBackup,
            ),
        ];

        for (swift, expected) in cases {
            let actual: DeviceTransferCommand = swift.into();
            assert_eq!(
                std::mem::discriminant(&actual),
                std::mem::discriminant(&expected)
            );
        }
    }
}

impl From<AccountProfile> for SwiftAccountProfile {
    fn from(value: AccountProfile) -> Self {
        Self {
            user_id: value.user_id,
            full_name: value.full_name,
            avatar_url: value.avatar_url,
            identity_number: value.identity_number,
            biography: value.biography,
            phone: value.phone,
            created_at: value.created_at,
            is_verified: value.is_verified,
            fiat_currency: value.fiat_currency,
            membership: value.membership,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftCircleItem {
    pub circle_id: String,
    pub name: String,
    pub conversation_count: i64,
}

impl From<CircleItem> for SwiftCircleItem {
    fn from(value: CircleItem) -> Self {
        Self {
            circle_id: value.circle_id,
            name: value.name,
            conversation_count: value.conversation_count,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftConversationUnseenCount {
    pub category: String,
    pub circle_id: Option<String>,
    pub count: i64,
    pub muted_count: i64,
}

impl From<ConversationUnseenCount> for SwiftConversationUnseenCount {
    fn from(value: ConversationUnseenCount) -> Self {
        Self {
            category: value.category,
            circle_id: value.circle_id,
            count: value.count,
            muted_count: value.muted_count,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftConversationChangeEvent {
    pub conversation_ids: Vec<String>,
    pub reload_all: bool,
}

impl From<ConversationChangeEvent> for SwiftConversationChangeEvent {
    fn from(value: ConversationChangeEvent) -> Self {
        Self {
            conversation_ids: value.conversation_ids,
            reload_all: value.reload_all,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftConversationListItem {
    pub conversation_id: String,
    pub owner_id: String,
    pub category: String,
    pub name: String,
    pub icon_url: String,
    pub draft: String,
    pub status: i32,
    pub last_message: String,
    pub last_message_category: Option<String>,
    pub last_message_status: Option<String>,
    pub last_message_sender_id: Option<String>,
    pub last_message_sender_name: Option<String>,
    pub last_message_action: Option<String>,
    pub last_message_participant_id: Option<String>,
    pub last_message_participant_name: Option<String>,
    pub last_read_message_id: Option<String>,
    pub unseen_count: i64,
    pub mention_count: i64,
    pub is_pinned: bool,
    pub pin_time_millis: i64,
    pub is_muted: bool,
    pub is_verified: bool,
    pub is_bot: bool,
    pub is_bot_group: bool,
    pub is_scam: bool,
    pub membership: Option<String>,
    pub relationship: String,
    pub identity_number: String,
    pub circle_ids: Vec<String>,
    pub participant_count: i64,
    pub group_avatars: Vec<SwiftGroupAvatar>,
    pub updated_at_millis: i64,
}

impl From<ConversationListData> for SwiftConversationListItem {
    fn from(value: ConversationListData) -> Self {
        Self {
            conversation_id: value.conversation_id,
            owner_id: value.owner_id,
            category: value.category,
            name: value.name,
            icon_url: value.avatar_url,
            draft: value.draft,
            status: value.status,
            last_message: value.last_message,
            last_message_category: value.last_message_category,
            last_message_status: value.last_message_status,
            last_message_sender_id: value.last_message_sender_id,
            last_message_sender_name: value.last_message_sender_name,
            last_message_action: value.last_message_action,
            last_message_participant_id: value.last_message_participant_id,
            last_message_participant_name: value.last_message_participant_name,
            last_read_message_id: value.last_read_message_id,
            unseen_count: value.unseen_count,
            mention_count: value.mention_count,
            is_pinned: value.is_pinned,
            pin_time_millis: value.pin_time_millis,
            is_muted: value.is_muted,
            is_verified: value.is_verified,
            is_bot: value.is_bot,
            is_bot_group: value.is_bot_group,
            is_scam: value.is_scam,
            membership: value.membership,
            relationship: value.relationship,
            identity_number: value.identity_number,
            circle_ids: value.circle_ids,
            participant_count: value.participant_count,
            group_avatars: value.group_avatars.into_iter().map(Into::into).collect(),
            updated_at_millis: value.updated_at_millis,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftGroupConversationItem {
    pub conversation_id: String,
    pub name: String,
    pub avatar_url: String,
    pub participant_count: i64,
}

impl From<mixin_desktop_api::GroupConversationItem> for SwiftGroupConversationItem {
    fn from(value: mixin_desktop_api::GroupConversationItem) -> Self {
        Self {
            conversation_id: value.conversation_id,
            name: value.name,
            avatar_url: value.avatar_url,
            participant_count: value.participant_count,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftGroupAvatar {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
}

impl From<GroupAvatar> for SwiftGroupAvatar {
    fn from(value: GroupAvatar) -> Self {
        Self {
            user_id: value.user_id,
            name: value.name,
            avatar_url: value.avatar_url,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftCodeResult {
    pub kind: String,
    pub user_id: Option<String>,
    pub conversation_id: Option<String>,
    pub conversation_name: Option<String>,
    pub participant_count: i64,
    pub participant_avatars: Vec<SwiftGroupAvatar>,
    pub already_member: bool,
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

impl From<CodeResult> for SwiftCodeResult {
    fn from(value: CodeResult) -> Self {
        Self {
            kind: value.kind,
            user_id: value.user_id,
            conversation_id: value.conversation_id,
            conversation_name: value.conversation_name,
            participant_count: value.participant_count,
            participant_avatars: value
                .participant_avatars
                .into_iter()
                .map(Into::into)
                .collect(),
            already_member: value.already_member,
            asset_symbol: value.asset_symbol,
            asset_icon_url: value.asset_icon_url,
            chain_icon_url: value.chain_icon_url,
            amount: value.amount,
            senders: value.senders,
            receivers: value.receivers,
            threshold: value.threshold,
            state: value.state,
            action: value.action,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftSnapshotDetailItem {
    pub snapshot_id: String,
    pub trace_id: Option<String>,
    pub snapshot_type: String,
    pub amount: String,
    pub created_at_millis: i64,
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

impl From<SnapshotDetailItem> for SwiftSnapshotDetailItem {
    fn from(value: SnapshotDetailItem) -> Self {
        Self {
            snapshot_id: value.snapshot_id,
            trace_id: value.trace_id,
            snapshot_type: value.snapshot_type,
            amount: value.amount,
            created_at_millis: value.created_at_millis,
            opponent_name: value.opponent_name,
            transaction_hash: value.transaction_hash,
            sender: value.sender,
            receiver: value.receiver,
            memo: value.memo,
            confirmations: value.confirmations,
            snapshot_hash: value.snapshot_hash,
            opening_balance: value.opening_balance,
            closing_balance: value.closing_balance,
            symbol: value.symbol,
            asset_name: value.asset_name,
            asset_icon_url: value.asset_icon_url,
            chain_icon_url: value.chain_icon_url,
            asset_confirmations: value.asset_confirmations,
            asset_tag: value.asset_tag,
            current_user_name: value.current_user_name,
            is_safe: value.is_safe,
            price_usd: value.price_usd,
            fiat_rate: value.fiat_rate,
            ticker_price_usd: value.ticker_price_usd,
            deposit_hash: value.deposit_hash,
            withdrawal_hash: value.withdrawal_hash,
            withdrawal_receiver: value.withdrawal_receiver,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftMessageItem {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_identity_number: Option<String>,
    pub sender_avatar_url: String,
    pub sender_relationship: String,
    pub sender_app_id: Option<String>,
    pub sender_is_bot: bool,
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

impl From<MessageListView> for SwiftMessageItem {
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

#[derive(Clone, uniffi::Record)]
pub struct SwiftSharedAppItem {
    pub app_id: String,
    pub name: String,
    pub icon_url: String,
    pub description: String,
    pub home_uri: String,
}

impl From<SharedAppItem> for SwiftSharedAppItem {
    fn from(value: SharedAppItem) -> Self {
        Self {
            app_id: value.app_id,
            name: value.name,
            icon_url: value.icon_url,
            description: value.description,
            home_uri: value.home_uri,
        }
    }
}

#[cfg(test)]
mod shared_app_tests {
    use super::*;

    #[test]
    fn shared_app_conversion_preserves_launch_and_presentation_fields() {
        let item = SwiftSharedAppItem::from(SharedAppItem {
            app_id: "app-id".to_owned(),
            name: "Shared App".to_owned(),
            icon_url: "https://example.com/icon.png".to_owned(),
            description: "Description".to_owned(),
            home_uri: "https://example.com/home".to_owned(),
        });

        assert_eq!(item.app_id, "app-id");
        assert_eq!(item.name, "Shared App");
        assert_eq!(item.icon_url, "https://example.com/icon.png");
        assert_eq!(item.description, "Description");
        assert_eq!(item.home_uri, "https://example.com/home");
    }
}

#[derive(Clone, uniffi::Record)]
pub struct SwiftStickerItem {
    pub sticker_id: String,
    pub album_id: Option<String>,
    pub name: String,
    pub asset_url: String,
    pub asset_width: i32,
    pub asset_height: i32,
    pub asset_type: String,
}

impl From<StickerItem> for SwiftStickerItem {
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

#[derive(Clone, uniffi::Record)]
pub struct SwiftStickerAlbumItem {
    pub album_id: String,
    pub name: String,
    pub icon_url: String,
    pub category: String,
    pub description: String,
    pub banner: Option<String>,
    pub added: bool,
    pub is_verified: bool,
}

impl From<StickerAlbumItem> for SwiftStickerAlbumItem {
    fn from(value: StickerAlbumItem) -> Self {
        Self {
            album_id: value.album_id,
            name: value.name,
            icon_url: value.icon_url,
            category: value.category,
            description: value.description,
            banner: value.banner,
            added: value.added,
            is_verified: value.is_verified,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftStickerAlbumSection {
    pub album: SwiftStickerAlbumItem,
    pub stickers: Vec<SwiftStickerItem>,
}

#[derive(uniffi::Record)]
pub struct SwiftStickerLibrary {
    pub recent: Vec<SwiftStickerItem>,
    pub personal: Vec<SwiftStickerItem>,
    pub albums: Vec<SwiftStickerAlbumSection>,
}

#[derive(uniffi::Record)]
pub struct SwiftStickerDetailItem {
    pub sticker: SwiftStickerItem,
    pub album: Option<SwiftStickerAlbumItem>,
    pub album_stickers: Vec<SwiftStickerItem>,
    pub is_personal: bool,
}

impl From<StickerDetailItem> for SwiftStickerDetailItem {
    fn from(value: StickerDetailItem) -> Self {
        Self {
            sticker: value.sticker.into(),
            album: value.album.map(Into::into),
            album_stickers: value.album_stickers.into_iter().map(Into::into).collect(),
            is_personal: value.is_personal,
        }
    }
}

#[cfg(test)]
mod sticker_tests {
    use mixin_desktop_api::{StickerAlbumItem, StickerDetailItem, StickerItem};

    use super::SwiftStickerDetailItem;

    #[test]
    fn sticker_detail_conversion_preserves_album_and_media_contract() {
        let sticker = StickerItem {
            sticker_id: "sticker-id".to_string(),
            album_id: Some("album-id".to_string()),
            name: "Wave".to_string(),
            asset_url: "https://example.com/wave.webp".to_string(),
            asset_width: 256,
            asset_height: 128,
            asset_type: "webp".to_string(),
            created_at_millis: 42,
            last_use_at_millis: Some(84),
        };
        let detail = SwiftStickerDetailItem::from(StickerDetailItem {
            sticker: sticker.clone(),
            album: Some(StickerAlbumItem {
                album_id: "album-id".to_string(),
                name: "Greetings".to_string(),
                icon_url: "https://example.com/icon.png".to_string(),
                category: "SYSTEM".to_string(),
                description: "Greeting stickers".to_string(),
                banner: None,
                added: true,
                is_verified: true,
            }),
            album_stickers: vec![sticker],
            is_personal: false,
        });

        assert_eq!(detail.sticker.sticker_id, "sticker-id");
        assert_eq!(detail.sticker.asset_type, "webp");
        assert_eq!(detail.sticker.asset_width, 256);
        assert_eq!(detail.album.as_ref().map(|album| album.added), Some(true));
        assert_eq!(detail.album_stickers.len(), 1);
        assert!(!detail.is_personal);
    }
}

#[derive(uniffi::Record)]
pub struct SwiftUserItem {
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

impl From<UserProfileItem> for SwiftUserItem {
    fn from(value: UserProfileItem) -> Self {
        Self {
            user_id: value.user_id,
            identity_number: value.identity_number,
            full_name: value.full_name,
            avatar_url: value.avatar_url,
            biography: value.biography,
            is_verified: value.is_verified,
            is_bot: value.is_bot,
            relationship: value.relationship,
            code_url: value.code_url,
            membership: value.membership,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftConversationDetailItem {
    pub conversation_id: String,
    pub name: String,
    pub announcement: String,
    pub code_url: String,
    pub created_at_millis: i64,
    pub mute_until_millis: i64,
    pub expire_in: i64,
}

impl From<ConversationDetailItem> for SwiftConversationDetailItem {
    fn from(value: ConversationDetailItem) -> Self {
        Self {
            conversation_id: value.conversation_id,
            name: value.name,
            announcement: value.announcement,
            code_url: value.code_url,
            created_at_millis: value.created_at_millis,
            mute_until_millis: value.mute_until_millis,
            expire_in: value.expire_in,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftConversationParticipantItem {
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

impl From<ConversationParticipantItem> for SwiftConversationParticipantItem {
    fn from(value: ConversationParticipantItem) -> Self {
        Self {
            user_id: value.user_id,
            role: value.role,
            created_at_millis: value.created_at_millis,
            identity_number: value.identity_number,
            full_name: value.full_name,
            avatar_url: value.avatar_url,
            biography: value.biography,
            is_verified: value.is_verified,
            is_bot: value.is_bot,
            relationship: value.relationship,
            membership: value.membership,
        }
    }
}

#[derive(uniffi::Enum)]
pub enum SwiftParticipantAction {
    Add,
    Remove,
    MakeAdmin,
    DismissAdmin,
}

impl SwiftParticipantAction {
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
    use super::SwiftParticipantAction;

    #[test]
    fn participant_actions_map_to_supported_api_updates() {
        let cases = [
            (SwiftParticipantAction::Add, "ADD", None),
            (SwiftParticipantAction::Remove, "REMOVE", None),
            (SwiftParticipantAction::MakeAdmin, "ROLE", Some("ADMIN")),
            (SwiftParticipantAction::DismissAdmin, "ROLE", None),
        ];

        for (action, expected_action, expected_role) in cases {
            let (api_action, role) = action.api_update();
            assert_eq!(api_action, expected_action);
            assert_eq!(role, expected_role);
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftConversationStorageUsage {
    pub conversation: SwiftConversationListItem,
    pub size_bytes: i64,
}

impl From<ConversationStorageUsage> for SwiftConversationStorageUsage {
    fn from(value: ConversationStorageUsage) -> Self {
        Self {
            conversation: value.conversation.into(),
            size_bytes: value.size_bytes,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftStorageCategoryUsage {
    pub category: String,
    pub size_bytes: i64,
}

impl From<StorageCategoryUsage> for SwiftStorageCategoryUsage {
    fn from(value: StorageCategoryUsage) -> Self {
        Self {
            category: value.category,
            size_bytes: value.size_bytes,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftProxyItem {
    pub id: String,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl From<ProxyItem> for SwiftProxyItem {
    fn from(value: ProxyItem) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            host: value.host,
            port: value.port,
            username: value.username,
            password: value.password,
        }
    }
}

impl From<SwiftProxyItem> for ProxyItem {
    fn from(value: SwiftProxyItem) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            host: value.host,
            port: value.port,
            username: value.username,
            password: value.password,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftProxySettings {
    pub enabled: bool,
    pub selected_proxy_id: Option<String>,
    pub proxies: Vec<SwiftProxyItem>,
}

impl From<ProxySettingsItem> for SwiftProxySettings {
    fn from(value: ProxySettingsItem) -> Self {
        Self {
            enabled: value.enabled,
            selected_proxy_id: value.selected_proxy_id,
            proxies: value.proxies.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SwiftProxySettings> for ProxySettingsItem {
    fn from(value: SwiftProxySettings) -> Self {
        Self {
            enabled: value.enabled,
            selected_proxy_id: value.selected_proxy_id,
            proxies: value.proxies.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftMcpSettings {
    pub enabled: bool,
    pub token: String,
    pub draft_tools_enabled: bool,
    pub circle_management_enabled: bool,
}

impl From<McpSettingsItem> for SwiftMcpSettings {
    fn from(value: McpSettingsItem) -> Self {
        Self {
            enabled: value.enabled,
            token: value.token,
            draft_tools_enabled: value.draft_tools_enabled,
            circle_management_enabled: value.circle_management_enabled,
        }
    }
}

impl From<SwiftMcpSettings> for McpSettingsItem {
    fn from(value: SwiftMcpSettings) -> Self {
        Self {
            enabled: value.enabled,
            token: value.token,
            draft_tools_enabled: value.draft_tools_enabled,
            circle_management_enabled: value.circle_management_enabled,
        }
    }
}

#[derive(uniffi::Record)]
pub struct SwiftMcpServerStatus {
    pub running: bool,
    pub endpoint: Option<String>,
    pub last_error: Option<String>,
}

impl From<McpServerStatusItem> for SwiftMcpServerStatus {
    fn from(value: McpServerStatusItem) -> Self {
        Self {
            running: value.running,
            endpoint: value.endpoint,
            last_error: value.last_error,
        }
    }
}

#[cfg(test)]
mod mcp_tests {
    use mixin_desktop_api::McpSettingsItem;

    use super::SwiftMcpSettings;

    #[test]
    fn mcp_settings_round_trip_preserves_server_and_permission_state() {
        let source = McpSettingsItem {
            enabled: true,
            token: "bearer-token".to_string(),
            draft_tools_enabled: true,
            circle_management_enabled: false,
        };

        let restored = McpSettingsItem::from(SwiftMcpSettings::from(source));

        assert!(restored.enabled);
        assert_eq!(restored.token, "bearer-token");
        assert!(restored.draft_tools_enabled);
        assert!(!restored.circle_management_enabled);
    }
}

#[cfg(test)]
mod proxy_tests {
    use mixin_desktop_api::{ProxyItem, ProxySettingsItem};

    use super::{SwiftProxyItem, SwiftProxySettings};

    #[test]
    fn proxy_settings_round_trip_preserves_credentials_and_selection() {
        let source = ProxySettingsItem {
            enabled: true,
            selected_proxy_id: Some("proxy-id".to_string()),
            proxies: vec![ProxyItem {
                id: "proxy-id".to_string(),
                kind: "socks5".to_string(),
                host: "127.0.0.1".to_string(),
                port: 1080,
                username: Some("user".to_string()),
                password: Some("secret".to_string()),
            }],
        };

        let swift = SwiftProxySettings::from(source);
        let restored = ProxySettingsItem::from(swift);

        assert!(restored.enabled);
        assert_eq!(restored.selected_proxy_id.as_deref(), Some("proxy-id"));
        let proxy = restored.proxies.into_iter().next().unwrap();
        assert_eq!(proxy.id, "proxy-id");
        assert_eq!(proxy.kind, "socks5");
        assert_eq!(proxy.host, "127.0.0.1");
        assert_eq!(proxy.port, 1080);
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("secret"));
    }

    #[test]
    fn swift_proxy_item_converts_to_public_api_item() {
        let item = SwiftProxyItem {
            id: "proxy-id".to_string(),
            kind: "http".to_string(),
            host: "localhost".to_string(),
            port: 8080,
            username: None,
            password: None,
        };

        let item = ProxyItem::from(item);

        assert_eq!(item.id, "proxy-id");
        assert_eq!(item.kind, "http");
        assert_eq!(item.host, "localhost");
        assert_eq!(item.port, 8080);
    }
}
