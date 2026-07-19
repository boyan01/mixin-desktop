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

pub struct ConversationChangeEvent {
    pub conversation_ids: Vec<String>,
    pub reload_all: bool,
}

pub struct ConversationUnseenCount {
    pub category: String,
    pub circle_id: Option<String>,
    pub count: i64,
    pub muted_count: i64,
}

pub struct GroupAvatar {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
}

pub struct PinMessagePreviewItem {
    pub message_id: String,
    pub content: String,
    pub sender_name: String,
}

pub struct ConversationStorageUsage {
    pub conversation: ConversationListData,
    pub size_bytes: i64,
}

pub struct StorageCategoryUsage {
    pub category: String,
    pub size_bytes: i64,
}

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

pub struct CircleItem {
    pub circle_id: String,
    pub name: String,
    pub conversation_count: i64,
}

pub struct ConversationDetailItem {
    pub conversation_id: String,
    pub name: String,
    pub announcement: String,
    pub code_url: String,
    pub created_at_millis: i64,
    pub mute_until_millis: i64,
    pub expire_in: i64,
}

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

impl SnapshotDetailItem {
    pub(crate) fn from_detail(
        detail: crate::db::mixin::snapshot::SnapshotDetail,
        current_user_name: String,
        ticker_price_usd: Option<String>,
    ) -> Self {
        Self {
            snapshot_id: detail.snapshot_id,
            trace_id: detail.trace_id,
            snapshot_type: detail.type_field,
            asset_id: detail.asset_id,
            amount: detail.amount,
            created_at_millis: detail.created_at.timestamp_millis(),
            opponent_id: detail.opponent_id,
            opponent_name: detail.opponent_name,
            transaction_hash: detail.transaction_hash,
            sender: detail.sender,
            receiver: detail.receiver,
            memo: detail.memo,
            confirmations: detail.confirmations,
            snapshot_hash: detail.snapshot_hash,
            opening_balance: detail.opening_balance,
            closing_balance: detail.closing_balance,
            symbol: detail.symbol.unwrap_or_default(),
            asset_name: detail.asset_name.unwrap_or_default(),
            asset_icon_url: detail.asset_icon_url.unwrap_or_default(),
            chain_icon_url: detail.chain_icon_url.unwrap_or_default(),
            asset_confirmations: detail.asset_confirmations.unwrap_or_default(),
            asset_tag: detail.asset_tag,
            current_user_name,
            is_safe: false,
            price_usd: detail.price_usd,
            fiat_rate: detail.fiat_rate,
            ticker_price_usd,
            deposit_hash: None,
            withdrawal_hash: None,
            withdrawal_receiver: None,
        }
    }

    pub(crate) fn from_safe_detail(
        detail: crate::db::mixin::safe_snapshot::SafeSnapshotDetail,
        current_user_name: String,
        ticker_price_usd: Option<String>,
    ) -> Self {
        let withdrawal = detail
            .withdrawal
            .as_deref()
            .and_then(|value| serde_json::from_str::<Option<sdk::SafeWithdrawal>>(value).ok())
            .flatten();
        let deposit = detail
            .deposit
            .as_deref()
            .and_then(|value| serde_json::from_str::<Option<sdk::SafeDeposit>>(value).ok())
            .flatten();
        let snapshot_type = if detail.type_field == "pending" {
            "pending".to_string()
        } else if withdrawal.is_some() {
            "withdrawal".to_string()
        } else if deposit.is_some() {
            "deposit".to_string()
        } else {
            "transfer".to_string()
        };
        Self {
            snapshot_id: detail.snapshot_id,
            trace_id: detail.trace_id,
            snapshot_type,
            asset_id: detail.asset_id,
            amount: detail.amount,
            created_at_millis: detail.created_at.timestamp_millis(),
            opponent_id: Some(detail.opponent_id),
            opponent_name: detail.opponent_name,
            transaction_hash: Some(detail.transaction_hash),
            sender: None,
            receiver: withdrawal.as_ref().map(|value| value.receiver.clone()),
            memo: Some(detail.memo),
            confirmations: detail.confirmations,
            snapshot_hash: None,
            opening_balance: detail.opening_balance,
            closing_balance: detail.closing_balance,
            symbol: detail.symbol.unwrap_or_default(),
            asset_name: detail.asset_name.unwrap_or_default(),
            asset_icon_url: detail.asset_icon_url.unwrap_or_default(),
            chain_icon_url: detail.chain_icon_url.unwrap_or_default(),
            asset_confirmations: detail.asset_confirmations.unwrap_or_default(),
            asset_tag: None,
            current_user_name,
            is_safe: true,
            price_usd: detail.price_usd,
            fiat_rate: detail.fiat_rate,
            ticker_price_usd,
            deposit_hash: deposit.map(|value| value.deposit_hash),
            withdrawal_hash: withdrawal
                .as_ref()
                .map(|value| value.withdrawal_hash.clone()),
            withdrawal_receiver: withdrawal.map(|value| value.receiver),
        }
    }
}

pub struct GroupConversationItem {
    pub conversation_id: String,
    pub name: String,
    pub avatar_url: String,
    pub participant_count: i64,
}

pub struct SharedAppItem {
    pub app_id: String,
    pub name: String,
    pub icon_url: String,
    pub description: String,
    pub home_uri: String,
}

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

pub struct StickerDetailItem {
    pub sticker: StickerItem,
    pub album: Option<StickerAlbumItem>,
    pub album_stickers: Vec<StickerItem>,
    pub is_personal: bool,
}

#[derive(Clone)]
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

pub struct NotificationEventBatch {
    pub events: Vec<NotificationEvent>,
    pub next_created_at_micros: i64,
    pub next_row_id: i64,
    pub has_more: bool,
}

impl NotificationEvent {
    pub(crate) fn from_message(
        value: crate::db::mixin::message::NotificationMessageItem,
        current_user_id: &str,
        current_identity_number: &str,
    ) -> Option<Self> {
        let dismiss_message_id = (value.category == sdk::message_category::MESSAGE_RECALL)
            .then(|| value.content.clone().unwrap_or_default());
        let mentioned = value.category.contains("TEXT")
            && mentions_identity(
                value.content.as_deref().unwrap_or_default(),
                current_identity_number,
            );
        let quoted = value
            .quote_content
            .as_deref()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
            .is_some_and(|quote| {
                quote.get("user_id").and_then(serde_json::Value::as_str) == Some(current_user_id)
            });
        if dismiss_message_id.is_none() && value.is_muted && !mentioned && !quoted {
            return None;
        }
        Some(Self {
            message_id: value.message_id,
            conversation_id: value.conversation_id,
            sender_name: value.sender_name,
            category: value.category,
            content: value.content.unwrap_or_default(),
            created_at_micros: value.created_at.and_utc().timestamp_micros(),
            conversation_name: value.conversation_name,
            conversation_category: value.conversation_category,
            dismiss_message_id,
        })
    }
}

fn mentions_identity(content: &str, identity_number: &str) -> bool {
    if identity_number.is_empty() {
        return false;
    }
    let mention = format!("@{identity_number}");
    content.match_indices(&mention).any(|(index, _)| {
        content[index + mention.len()..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_digit())
    })
}

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

#[cfg(test)]
mod notification_tests {
    use chrono::DateTime;

    use super::NotificationEvent;
    use crate::db::mixin::message::NotificationMessageItem;

    fn message(
        category: &str,
        content: &str,
        quote_content: Option<&str>,
        is_muted: bool,
    ) -> NotificationMessageItem {
        NotificationMessageItem {
            row_id: 1,
            message_id: "message".to_string(),
            conversation_id: "conversation".to_string(),
            user_id: "sender".to_string(),
            sender_name: "Sender".to_string(),
            category: category.to_string(),
            content: Some(content.to_string()),
            quote_content: quote_content.map(str::to_string),
            created_at: DateTime::UNIX_EPOCH.naive_utc(),
            conversation_name: "Conversation".to_string(),
            conversation_category: "GROUP".to_string(),
            is_muted,
        }
    }

    #[test]
    fn filters_muted_messages_without_a_mention_or_quote() {
        let event = NotificationEvent::from_message(
            message("SIGNAL_TEXT", "hello @70001", None, true),
            "current-user",
            "7000",
        );

        assert!(event.is_none());
    }

    #[test]
    fn keeps_muted_messages_that_mention_or_quote_the_current_user() {
        let mention = NotificationEvent::from_message(
            message("SIGNAL_TEXT", "hello @7000", None, true),
            "current-user",
            "7000",
        );
        let quote = NotificationEvent::from_message(
            message(
                "SIGNAL_TEXT",
                "reply",
                Some(r#"{"user_id":"current-user"}"#),
                true,
            ),
            "current-user",
            "7000",
        );

        assert!(mention.is_some());
        assert!(quote.is_some());
    }

    #[test]
    fn converts_recall_messages_to_dismiss_events_even_when_muted() {
        let event = NotificationEvent::from_message(
            message("MESSAGE_RECALL", "recalled-message", None, true),
            "current-user",
            "7000",
        )
        .unwrap();

        assert_eq!(
            event.dismiss_message_id.as_deref(),
            Some("recalled-message")
        );
    }
}
