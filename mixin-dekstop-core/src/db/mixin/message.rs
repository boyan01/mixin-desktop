use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sdk::blaze_message::MessageStatus;

use crate::db::Error;

pub struct MessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Default)]
pub struct Message {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub category: String,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_duration: String,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub media_hash: Option<String>,
    pub thumb_image: Option<String>,
    pub media_key: Option<Vec<u8>>,
    pub media_digest: Option<Vec<u8>>,
    pub media_status: MediaStatus,
    pub status: MessageStatus,
    pub created_at: NaiveDateTime,
    pub action: Option<String>,
    pub participant_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub hyperlink: Option<String>,
    pub name: Option<String>,
    pub album_id: Option<String>,
    pub sticker_id: Option<String>,
    pub shared_user_id: Option<String>,
    pub media_waveform: Option<String>,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub thumb_url: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[derive(sqlx::Type)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaStatus {
    Pending,
    Done,
    #[default]
    Canceled,
    Expired,
    Read,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[derive(Serialize, Deserialize)]
#[derive(sqlx::FromRow)]
pub struct QuoteMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub user_full_name: Option<String>,
    pub user_identity_number: String,
    pub app_id: Option<String>,
    pub category: String,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: MessageStatus,
    pub media_status: Option<MediaStatus>,
    pub media_waveform: Option<String>,
    pub media_name: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i32>,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub thumb_image: Option<String>,
    pub thumb_url: Option<String>,
    pub media_url: Option<String>,
    pub media_duration: Option<String>,
    pub sticker_id: Option<String>,
    pub asset_url: Option<String>,
    pub asset_width: Option<i32>,
    pub asset_height: Option<i32>,
    pub asset_name: Option<String>,
    pub asset_type: Option<String>,
    pub shared_user_id: Option<String>,
    pub shared_user_full_name: Option<String>,
    pub shared_user_identity_number: Option<String>,
    pub shared_user_avatar_url: Option<String>,
    pub shared_user_is_verified: Option<bool>,
    pub shared_user_app_id: Option<String>,
}

const QUOTE_MESSAGE_QUERY_PREFIX: &str = r#"
SELECT message.message_id AS messageId, message.conversation_id AS conversationId,
    sender.user_id AS userId,
    sender.full_name AS userFullName, sender.identity_number AS userIdentityNumber,
    sender.app_id AS appId,
    message.category AS type,
    message.content AS content, message.created_at AS createdAt, message.status AS status,
    message.media_status AS mediaStatus, message.media_waveform AS mediaWaveform,
    message.name AS mediaName, message.media_mime_type AS mediaMimeType,
    message.media_size AS mediaSize,
    message.media_width AS mediaWidth, message.media_height AS mediaHeight,
    message.thumb_image AS thumbImage, message.thumb_url AS thumbUrl, message.media_url AS mediaUrl,
    message.media_duration AS mediaDuration,
    message.sticker_id AS stickerId,
    sticker.asset_url AS assetUrl, sticker.asset_width AS assetWidth,
    sticker.asset_height AS assetHeight,
    sticker.name AS assetName, sticker.asset_type AS assetType,
    message.shared_user_id AS sharedUserId,
    shareUser.full_name AS sharedUserFullName,
    shareUser.identity_number AS sharedUserIdentityNumber,
    shareUser.avatar_url AS sharedUserAvatarUrl, shareUser.is_verified AS sharedUserIsVerified,
    shareUser.app_id AS sharedUserAppId
FROM messages message
         INNER JOIN users sender ON message.user_id = sender.user_id
         LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
         LEFT JOIN users shareUser ON message.shared_user_id = shareUser.user_id
         LEFT JOIN message_mentions messageMention ON message.message_id = messageMention.message_id
"#;

impl MessageDao {
    pub async fn find_quote_message_by_id(&self, message_id: &String) -> Result<Option<QuoteMessage>, Error> {
        let query_str = format!("{} WHERE message.message_id = ?", QUOTE_MESSAGE_QUERY_PREFIX);
        let result = sqlx::query_as::<_, QuoteMessage>(&query_str).bind(message_id).fetch_optional(&self.0).await?;
        Ok(result)
    }

    pub async fn is_message_exits(&self, message_id: &String) -> Result<bool, Error> {
        todo!()
    }

    pub async fn insert_message(&self, message: &Message) -> Result<(), Error> {
        todo!()
    }
}
