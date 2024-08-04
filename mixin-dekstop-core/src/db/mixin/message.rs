use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};

use sdk::blaze_message::MessageStatus;

use crate::db::Error;
use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::util::{BindList, BindListForQuery, expand_var};

#[derive(Clone)]
pub struct MessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Default, sqlx::FromRow)]
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

#[derive(Debug, PartialEq, Eq, Clone, Default, sqlx::Type, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    pub media_size: Option<i64>,
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

pub struct AttachmentMessageUpdate {
    pub status: MessageStatus,
    pub content: String,
    pub media_mine_type: String,
    pub media_size: i64,
    pub media_status: MediaStatus,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub media_digest: Option<Vec<u8>>,
    pub media_key: Option<Vec<u8>>,
    pub media_waveform: Option<Vec<u8>>,
    pub caption: Option<String>,
    pub name: Option<String>,
    pub thumb_image: Option<String>,
    pub media_duration: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MiniMessageItem {
    pub message_id: String,
    pub conversation_id: String,
}

impl MessageDao {
    pub async fn find_quote_message_by_id(
        &self,
        message_id: &str,
    ) -> Result<Option<QuoteMessage>, Error> {
        let query_str = format!(
            "{} WHERE message.message_id = ?",
            QUOTE_MESSAGE_QUERY_PREFIX
        );
        let result = sqlx::query_as::<_, QuoteMessage>(&query_str)
            .bind(message_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn find_message_by_id(&self, message_id: &String) -> Result<Option<Message>, Error> {
        let result = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE message_id = ?")
            .bind(message_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn is_message_exits(&self, message_id: &String) -> Result<bool, Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE message_id = ?)",
        )
        .bind(message_id)
        .fetch_one(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn insert_message(&self, message: &Message) -> Result<(), Error> {
        let _ = sqlx::query(r#"
INSERT OR REPLACE INTO messages (message_id, conversation_id, user_id, category, content,
media_url, media_mime_type, media_size, media_duration, media_width, media_height, media_hash,
thumb_image, media_key, media_digest, media_status, status, created_at, action, participant_id,
snapshot_id, hyperlink, name, album_id, sticker_id, shared_user_id, media_waveform, quote_message_id,
quote_content, thumb_url, caption)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#)
            .bind(&message.message_id)
            .bind(&message.conversation_id)
            .bind(&message.user_id)
            .bind(&message.category)
            .bind(&message.content)
            .bind(&message.media_url)
            .bind(&message.media_mime_type)
            .bind(message.media_size)
            .bind(&message.media_duration)
            .bind(message.media_width)
            .bind(message.media_height)
            .bind(&message.media_hash)
            .bind(&message.thumb_image)
            .bind(&message.media_key)
            .bind(&message.media_digest)
            .bind(&message.media_status)
            .bind(message.status)
            .bind(message.created_at)
            .bind(&message.action)
            .bind(&message.participant_id)
            .bind(&message.snapshot_id)
            .bind(&message.hyperlink)
            .bind(&message.name)
            .bind(&message.album_id)
            .bind(&message.sticker_id)
            .bind(&message.shared_user_id)
            .bind(&message.media_waveform)
            .bind(&message.quote_message_id)
            .bind(&message.quote_content)
            .bind(&message.thumb_url)
            .bind(&message.caption)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_message_content_and_status(
        &self,
        message_id: &str,
        content: &str,
        status: MessageStatus,
    ) -> Result<(), Error> {
        let _ = sqlx::query("UPDATE messages SET content = ?, status = ? WHERE message_id = ?")
            .bind(content)
            .bind(status)
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_attachment_message(
        &self,
        message_id: &str,
        update: &AttachmentMessageUpdate,
    ) -> Result<(), Error> {
        let _ = sqlx::query(
            r#"UPDATE messages SET
         status = ?, content = ?, media_mine_type = ?, media_size = ?, media_status = ?,
         media_width = ?, media_height = ?, media_digest = ?, media_key = ?, media_waveform = ?,
         caption = ?, name = ?, thumb_image = ?, media_duration = ?
          WHERE message_id = ?"#,
        )
        .bind(update.status)
        .bind(&update.content)
        .bind(&update.media_mine_type)
        .bind(update.media_size)
        .bind(&update.media_status)
        .bind(update.media_width)
        .bind(update.media_height)
        .bind(&update.media_digest)
        .bind(&update.media_key)
        .bind(&update.media_waveform)
        .bind(&update.caption)
        .bind(&update.name)
        .bind(&update.thumb_image)
        .bind(&update.media_duration)
        .bind(message_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub(crate) async fn update_sticker_message(
        &self,
        message_id: &str,
        sticker_id: String,
        status: MessageStatus,
    ) -> Result<(), Error> {
        let _ = sqlx::query("UPDATE messages SET sticker_id = ?, status = ? WHERE message_id = ?")
            .bind(sticker_id)
            .bind(status)
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub(crate) async fn update_contact_message(
        &self,
        message_id: &str,
        user_id: String,
        status: MessageStatus,
    ) -> Result<(), Error> {
        let _ =
            sqlx::query("UPDATE messages SET shared_user_id = ?, status = ? WHERE message_id = ?")
                .bind(user_id)
                .bind(status)
                .bind(message_id)
                .execute(&self.0)
                .await;
        Ok(())
    }

    pub async fn update_live_message(
        &self,
        message_id: &str,
        width: i32,
        height: i32,
        url: &str,
        thumb_url: &str,
        status: MessageStatus,
    ) -> Result<(), Error> {
        let _ = sqlx::query(
            "UPDATE messages SET media_width = ?, media_height = ?, media_url = ?, thumb_url = ?, status = ? WHERE message_id = ?",
        )
        .bind(width)
        .bind(height)
        .bind(url)
        .bind(thumb_url)
        .bind(status)
        .bind(message_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update_message_quote_if_need(
        &self,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<(), Error> {
        let message_ids = sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND quote_message_id = ?",
        )
        .bind(conversation_id)
        .bind(message_id)
        .fetch_all(&self.0)
        .await?;
        if message_ids.is_empty() {
            return Ok(());
        }

        let message = self.find_quote_message_by_id(message_id).await?;

        if let Some(message) = message {
            let content =
                serde_json::to_string(&message).with_context(|| "convert quote message to json")?;

            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "UPDATE messages SET quote_content = ? WHERE where message_id IN (",
            );

            for message_id in message_ids {
                query_builder.push_bind(message_id);
            }
            query_builder.push(")");
            let _ = query_builder.build().bind(&content).execute(&self.0).await;
        }

        Ok(())
    }

    pub async fn mini_message_by_ids(&self, ids: &[String]) -> Result<Vec<MiniMessageItem>, Error> {
        let query_str = format!(
            "SELECT conversation_id, message_id FROM messages WHERE message_id IN ({})",
            expand_var(ids.len())
        );
        let result = sqlx::query_as::<_, MiniMessageItem>(&query_str)
            .bind_list(ids)
            .fetch_all(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn mark_message_read(&self, messages: &[String]) -> Result<(), Error> {
        let mut iter = messages.chunks(MARK_LIMIT);
        while let Some(chunk) = iter.next() {
            let ids = chunk.iter().map(|m| m.as_str()).collect::<Vec<&str>>();
            let _ = sqlx::query(&format!(
                "UPDATE messages SET status = ? WHERE message_id in {}",
                expand_var(chunk.len())
            ))
            .bind(MessageStatus::Read)
            .bind_list(&ids)
            .execute(&self.0)
            .await?;
        }

        Ok(())
    }

    pub async fn find_failed_message(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<Vec<String>, Error> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND user_id = ? AND status = ? \
            ORDER BY created_at DESC LIMIT 1000",
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(MessageStatus::Failed)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_message_dao() {}
}
