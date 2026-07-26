//! Stable application-facing data transfer objects.

use mixin_desktop_core::runtime::model as core;

#[derive(Clone, Debug)]
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

impl From<core::ConversationListData> for ConversationListData {
    fn from(value: core::ConversationListData) -> Self {
        let core::ConversationListData {
            conversation_id,
            owner_id,
            name,
            avatar_url,
            category,
            draft,
            status,
            last_read_message_id,
            last_message,
            last_message_category,
            last_message_status,
            last_message_sender_id,
            last_message_sender_name,
            last_message_action,
            last_message_participant_id,
            last_message_participant_name,
            updated_at_millis,
            unseen_count,
            mention_count,
            is_muted,
            is_verified,
            is_scam,
            is_bot,
            is_bot_group,
            membership,
            is_pinned,
            pin_time_millis,
            relationship,
            identity_number,
            circle_ids,
            participant_count,
            group_avatars,
        } = value;
        Self {
            conversation_id,
            owner_id,
            name,
            avatar_url,
            category,
            draft,
            status,
            last_read_message_id,
            last_message,
            last_message_category,
            last_message_status,
            last_message_sender_id,
            last_message_sender_name,
            last_message_action,
            last_message_participant_id,
            last_message_participant_name,
            updated_at_millis,
            unseen_count,
            mention_count,
            is_muted,
            is_verified,
            is_scam,
            is_bot,
            is_bot_group,
            membership,
            is_pinned,
            pin_time_millis,
            relationship,
            identity_number,
            circle_ids,
            participant_count,
            group_avatars: group_avatars.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConversationUnseenCount {
    pub category: String,
    pub circle_id: Option<String>,
    pub count: i64,
    pub muted_count: i64,
}

impl From<core::ConversationUnseenCount> for ConversationUnseenCount {
    fn from(value: core::ConversationUnseenCount) -> Self {
        let core::ConversationUnseenCount {
            category,
            circle_id,
            count,
            muted_count,
        } = value;
        Self {
            category,
            circle_id,
            count,
            muted_count,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GroupAvatar {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
}

impl From<core::GroupAvatar> for GroupAvatar {
    fn from(value: core::GroupAvatar) -> Self {
        let core::GroupAvatar {
            user_id,
            name,
            avatar_url,
        } = value;
        Self {
            user_id,
            name,
            avatar_url,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PinMessagePreviewItem {
    pub message_id: String,
    pub content: String,
    pub sender_name: String,
}

impl From<core::PinMessagePreviewItem> for PinMessagePreviewItem {
    fn from(value: core::PinMessagePreviewItem) -> Self {
        let core::PinMessagePreviewItem {
            message_id,
            content,
            sender_name,
        } = value;
        Self {
            message_id,
            content,
            sender_name,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConversationStorageUsage {
    pub conversation: ConversationListData,
    pub size_bytes: i64,
}

impl From<core::ConversationStorageUsage> for ConversationStorageUsage {
    fn from(value: core::ConversationStorageUsage) -> Self {
        let core::ConversationStorageUsage {
            conversation,
            size_bytes,
        } = value;
        Self {
            conversation: conversation.into(),
            size_bytes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StorageCategoryUsage {
    pub category: String,
    pub size_bytes: i64,
}

impl From<core::StorageCategoryUsage> for StorageCategoryUsage {
    fn from(value: core::StorageCategoryUsage) -> Self {
        let core::StorageCategoryUsage {
            category,
            size_bytes,
        } = value;
        Self {
            category,
            size_bytes,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::UserProfileItem> for UserProfileItem {
    fn from(value: core::UserProfileItem) -> Self {
        let core::UserProfileItem {
            user_id,
            identity_number,
            full_name,
            avatar_url,
            biography,
            is_verified,
            is_bot,
            relationship,
            code_url,
            membership,
        } = value;
        Self {
            user_id,
            identity_number,
            full_name,
            avatar_url,
            biography,
            is_verified,
            is_bot,
            relationship,
            code_url,
            membership,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CircleItem {
    pub circle_id: String,
    pub name: String,
    pub conversation_count: i64,
}

impl From<core::CircleItem> for CircleItem {
    fn from(value: core::CircleItem) -> Self {
        let core::CircleItem {
            circle_id,
            name,
            conversation_count,
        } = value;
        Self {
            circle_id,
            name,
            conversation_count,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConversationDetailItem {
    pub conversation_id: String,
    pub name: String,
    pub announcement: String,
    pub code_url: String,
    pub created_at_millis: i64,
    pub mute_until_millis: i64,
    pub expire_in: i64,
}

impl From<core::ConversationDetailItem> for ConversationDetailItem {
    fn from(value: core::ConversationDetailItem) -> Self {
        let core::ConversationDetailItem {
            conversation_id,
            name,
            announcement,
            code_url,
            created_at_millis,
            mute_until_millis,
            expire_in,
        } = value;
        Self {
            conversation_id,
            name,
            announcement,
            code_url,
            created_at_millis,
            mute_until_millis,
            expire_in,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::CodeResult> for CodeResult {
    fn from(value: core::CodeResult) -> Self {
        let core::CodeResult {
            kind,
            user_id,
            conversation_id,
            conversation_name,
            participant_count,
            participant_avatars,
            already_member,
            asset_id,
            asset_symbol,
            asset_icon_url,
            chain_icon_url,
            amount,
            senders,
            receivers,
            threshold,
            state,
            action,
        } = value;
        Self {
            kind,
            user_id,
            conversation_id,
            conversation_name,
            participant_count,
            participant_avatars: participant_avatars.into_iter().map(Into::into).collect(),
            already_member,
            asset_id,
            asset_symbol,
            asset_icon_url,
            chain_icon_url,
            amount,
            senders,
            receivers,
            threshold,
            state,
            action,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::SnapshotDetailItem> for SnapshotDetailItem {
    fn from(value: core::SnapshotDetailItem) -> Self {
        let core::SnapshotDetailItem {
            snapshot_id,
            trace_id,
            snapshot_type,
            asset_id,
            amount,
            created_at_millis,
            opponent_id,
            opponent_name,
            transaction_hash,
            sender,
            receiver,
            memo,
            confirmations,
            snapshot_hash,
            opening_balance,
            closing_balance,
            symbol,
            asset_name,
            asset_icon_url,
            chain_icon_url,
            asset_confirmations,
            asset_tag,
            current_user_name,
            is_safe,
            price_usd,
            fiat_rate,
            ticker_price_usd,
            deposit_hash,
            withdrawal_hash,
            withdrawal_receiver,
        } = value;
        Self {
            snapshot_id,
            trace_id,
            snapshot_type,
            asset_id,
            amount,
            created_at_millis,
            opponent_id,
            opponent_name,
            transaction_hash,
            sender,
            receiver,
            memo,
            confirmations,
            snapshot_hash,
            opening_balance,
            closing_balance,
            symbol,
            asset_name,
            asset_icon_url,
            chain_icon_url,
            asset_confirmations,
            asset_tag,
            current_user_name,
            is_safe,
            price_usd,
            fiat_rate,
            ticker_price_usd,
            deposit_hash,
            withdrawal_hash,
            withdrawal_receiver,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GroupConversationItem {
    pub conversation_id: String,
    pub name: String,
    pub avatar_url: String,
    pub participant_count: i64,
}

impl From<core::GroupConversationItem> for GroupConversationItem {
    fn from(value: core::GroupConversationItem) -> Self {
        let core::GroupConversationItem {
            conversation_id,
            name,
            avatar_url,
            participant_count,
        } = value;
        Self {
            conversation_id,
            name,
            avatar_url,
            participant_count,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SharedAppItem {
    pub app_id: String,
    pub name: String,
    pub icon_url: String,
    pub description: String,
    pub home_uri: String,
}

impl From<core::SharedAppItem> for SharedAppItem {
    fn from(value: core::SharedAppItem) -> Self {
        let core::SharedAppItem {
            app_id,
            name,
            icon_url,
            description,
            home_uri,
        } = value;
        Self {
            app_id,
            name,
            icon_url,
            description,
            home_uri,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::ConversationParticipantItem> for ConversationParticipantItem {
    fn from(value: core::ConversationParticipantItem) -> Self {
        let core::ConversationParticipantItem {
            user_id,
            role,
            created_at_millis,
            identity_number,
            full_name,
            avatar_url,
            biography,
            is_verified,
            is_bot,
            relationship,
            membership,
        } = value;
        Self {
            user_id,
            role,
            created_at_millis,
            identity_number,
            full_name,
            avatar_url,
            biography,
            is_verified,
            is_bot,
            relationship,
            membership,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::StickerItem> for StickerItem {
    fn from(value: core::StickerItem) -> Self {
        let core::StickerItem {
            sticker_id,
            album_id,
            name,
            asset_url,
            asset_width,
            asset_height,
            asset_type,
            created_at_millis,
            last_use_at_millis,
        } = value;
        Self {
            sticker_id,
            album_id,
            name,
            asset_url,
            asset_width,
            asset_height,
            asset_type,
            created_at_millis,
            last_use_at_millis,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::StickerAlbumItem> for StickerAlbumItem {
    fn from(value: core::StickerAlbumItem) -> Self {
        let core::StickerAlbumItem {
            album_id,
            name,
            icon_url,
            category,
            description,
            banner,
            added,
            is_verified,
        } = value;
        Self {
            album_id,
            name,
            icon_url,
            category,
            description,
            banner,
            added,
            is_verified,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StickerDetailItem {
    pub sticker: StickerItem,
    pub album: Option<StickerAlbumItem>,
    pub album_stickers: Vec<StickerItem>,
    pub is_personal: bool,
}

impl From<core::StickerDetailItem> for StickerDetailItem {
    fn from(value: core::StickerDetailItem) -> Self {
        let core::StickerDetailItem {
            sticker,
            album,
            album_stickers,
            is_personal,
        } = value;
        Self {
            sticker: sticker.into(),
            album: album.map(Into::into),
            album_stickers: album_stickers.into_iter().map(Into::into).collect(),
            is_personal,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MessageListView {
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
    pub sender_is_scam: bool,
    pub sender_is_bot: bool,
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
    pub media_status: String,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub quote_user_membership: Option<String>,
    pub caption: Option<String>,
    pub action: Option<String>,
    pub participant_id: Option<String>,
    pub participant_full_name: Option<String>,
    pub snapshot_id: Option<String>,
    pub snapshot_type: Option<String>,
    pub snapshot_amount: Option<String>,
    pub snapshot_memo: Option<String>,
    pub snapshot_asset_id: Option<String>,
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
    pub hyperlink: Option<String>,
    pub media_name: Option<String>,
    pub album_id: Option<String>,
    pub sticker_id: Option<String>,
    pub shared_user_id: Option<String>,
    pub media_waveform: Option<String>,
    pub thumb_url: Option<String>,
    pub conversation_owner_id: Option<String>,
    pub conversation_category: Option<String>,
    pub shared_user_full_name: Option<String>,
    pub shared_user_identity_number: Option<String>,
    pub shared_user_avatar_url: Option<String>,
    pub shared_user_is_verified: bool,
    pub shared_user_membership: Option<String>,
    pub shared_user_app_id: Option<String>,
    pub sticker_asset_url: Option<String>,
    pub sticker_asset_width: Option<i32>,
    pub sticker_asset_height: Option<i32>,
    pub sticker_asset_name: Option<String>,
    pub sticker_asset_type: Option<String>,
    pub mention_read: Option<bool>,
    pub pinned: bool,
    pub expire_in: Option<i64>,
}

impl From<core::MessageListView> for MessageListView {
    fn from(value: core::MessageListView) -> Self {
        let core::MessageListView {
            message_id,
            conversation_id,
            sender_id,
            sender_name,
            sender_identity_number,
            sender_avatar_url,
            sender_is_verified,
            sender_membership,
            sender_relationship,
            sender_app_id,
            sender_is_scam,
            sender_is_bot,
            category,
            content,
            status,
            created_at_micros,
            media_url,
            media_mime_type,
            media_size,
            media_duration,
            media_width,
            media_height,
            thumb_image,
            media_status,
            quote_message_id,
            quote_content,
            quote_user_membership,
            caption,
            action,
            participant_id,
            participant_full_name,
            snapshot_id,
            snapshot_type,
            snapshot_amount,
            snapshot_memo,
            snapshot_asset_id,
            snapshot_asset_symbol,
            snapshot_asset_icon_url,
            snapshot_chain_icon_url,
            snapshot_opponent_id,
            snapshot_transaction_hash,
            snapshot_created_at,
            inscription_hash,
            inscription_collection_hash,
            inscription_sequence,
            inscription_content_type,
            inscription_content_url,
            inscription_name,
            inscription_icon_url,
            hyperlink,
            media_name,
            album_id,
            sticker_id,
            shared_user_id,
            media_waveform,
            thumb_url,
            conversation_owner_id,
            conversation_category,
            shared_user_full_name,
            shared_user_identity_number,
            shared_user_avatar_url,
            shared_user_is_verified,
            shared_user_membership,
            shared_user_app_id,
            sticker_asset_url,
            sticker_asset_width,
            sticker_asset_height,
            sticker_asset_name,
            sticker_asset_type,
            mention_read,
            pinned,
            expire_in,
        } = value;
        Self {
            message_id,
            conversation_id,
            sender_id,
            sender_name,
            sender_identity_number,
            sender_avatar_url,
            sender_is_verified,
            sender_membership,
            sender_relationship,
            sender_app_id,
            sender_is_scam,
            sender_is_bot,
            category,
            content,
            status,
            created_at_micros,
            media_url,
            media_mime_type,
            media_size,
            media_duration,
            media_width,
            media_height,
            thumb_image,
            media_status,
            quote_message_id,
            quote_content,
            quote_user_membership,
            caption,
            action,
            participant_id,
            participant_full_name,
            snapshot_id,
            snapshot_type,
            snapshot_amount,
            snapshot_memo,
            snapshot_asset_id,
            snapshot_asset_symbol,
            snapshot_asset_icon_url,
            snapshot_chain_icon_url,
            snapshot_opponent_id,
            snapshot_transaction_hash,
            snapshot_created_at,
            inscription_hash,
            inscription_collection_hash,
            inscription_sequence,
            inscription_content_type,
            inscription_content_url,
            inscription_name,
            inscription_icon_url,
            hyperlink,
            media_name,
            album_id,
            sticker_id,
            shared_user_id,
            media_waveform,
            thumb_url,
            conversation_owner_id,
            conversation_category,
            shared_user_full_name,
            shared_user_identity_number,
            shared_user_avatar_url,
            shared_user_is_verified,
            shared_user_membership,
            shared_user_app_id,
            sticker_asset_url,
            sticker_asset_width,
            sticker_asset_height,
            sticker_asset_name,
            sticker_asset_type,
            mention_read,
            pinned,
            expire_in,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MessageOrderInfoView {
    pub message_id: String,
    pub row_id: i64,
    pub created_at_micros: i64,
}

impl From<core::MessageOrderInfoView> for MessageOrderInfoView {
    fn from(value: core::MessageOrderInfoView) -> Self {
        let core::MessageOrderInfoView {
            message_id,
            row_id,
            created_at_micros,
        } = value;
        Self {
            message_id,
            row_id,
            created_at_micros,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::NotificationEvent> for NotificationEvent {
    fn from(value: core::NotificationEvent) -> Self {
        let core::NotificationEvent {
            message_id,
            conversation_id,
            sender_name,
            category,
            content,
            created_at_micros,
            conversation_name,
            conversation_category,
            dismiss_message_id,
        } = value;
        Self {
            message_id,
            conversation_id,
            sender_name,
            category,
            content,
            created_at_micros,
            conversation_name,
            conversation_category,
            dismiss_message_id,
        }
    }
}

#[derive(Clone, Debug)]
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

impl From<core::ImageMessageView> for ImageMessageView {
    fn from(value: core::ImageMessageView) -> Self {
        let core::ImageMessageView {
            message_id,
            created_at_micros,
            media_url,
            media_name,
            thumb_image,
            can_forward,
            user_id,
            user_full_name,
            user_identity_number,
            avatar_url,
        } = value;
        Self {
            message_id,
            created_at_micros,
            media_url,
            media_name,
            thumb_image,
            can_forward,
            user_id,
            user_full_name,
            user_identity_number,
            avatar_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use mixin_desktop_core::runtime::model as core;

    use super::{StickerDetailItem, UserProfileItem};

    #[test]
    fn user_profile_conversion_preserves_public_contract_fields() {
        let profile = UserProfileItem::from(core::UserProfileItem {
            user_id: "user-id".to_string(),
            identity_number: "7000".to_string(),
            full_name: "Mixin".to_string(),
            avatar_url: "https://example.com/avatar".to_string(),
            biography: "bio".to_string(),
            is_verified: true,
            is_bot: false,
            relationship: "FRIEND".to_string(),
            code_url: "mixin://users/user-id".to_string(),
            membership: Some("membership".to_string()),
        });

        assert_eq!(profile.user_id, "user-id");
        assert_eq!(profile.identity_number, "7000");
        assert_eq!(profile.full_name, "Mixin");
        assert!(profile.is_verified);
        assert_eq!(profile.membership.as_deref(), Some("membership"));
    }

    #[test]
    fn sticker_detail_conversion_maps_nested_album_and_stickers() {
        let sticker = core::StickerItem {
            sticker_id: "sticker-id".to_string(),
            album_id: Some("album-id".to_string()),
            name: "Sticker".to_string(),
            asset_url: "https://example.com/sticker".to_string(),
            asset_width: 128,
            asset_height: 128,
            asset_type: "png".to_string(),
            created_at_millis: 1,
            last_use_at_millis: Some(2),
        };
        let detail = StickerDetailItem::from(core::StickerDetailItem {
            sticker,
            album: Some(core::StickerAlbumItem {
                album_id: "album-id".to_string(),
                name: "Album".to_string(),
                icon_url: "https://example.com/icon".to_string(),
                category: "SYSTEM".to_string(),
                description: "description".to_string(),
                banner: None,
                added: true,
                is_verified: true,
            }),
            album_stickers: Vec::new(),
            is_personal: true,
        });

        assert_eq!(detail.sticker.sticker_id, "sticker-id");
        assert_eq!(
            detail.album.as_ref().map(|album| album.album_id.as_str()),
            Some("album-id")
        );
        assert!(detail.is_personal);
    }
}
