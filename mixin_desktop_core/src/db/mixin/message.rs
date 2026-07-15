use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};

use sdk::blaze_message::MessageStatus;
use sdk::message_category::{MESSAGE_PIN, MESSAGE_RECALL};

use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::util::{expand_var, BindList, BindListForQuery};
use crate::db::Error;

#[derive(Clone)]
pub struct MessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Clone, Default, sqlx::FromRow)]
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
#[serde(rename_all = "camelCase")]
#[sqlx(rename_all = "camelCase")]
pub struct QuoteMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub user_full_name: Option<String>,
    pub user_identity_number: String,
    pub app_id: Option<String>,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
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
    pub media_mime_type: String,
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
        let result = sqlx::query_as::<_, QuoteMessage>(sqlx::AssertSqlSafe(query_str))
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

    pub async fn update_transcript_message(
        &self,
        message_id: &str,
        content: &str,
        media_size: i64,
        media_status: MediaStatus,
        status: MessageStatus,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE messages SET content = ?, media_size = ?, media_status = ?, status = ? \
             WHERE message_id = ?",
        )
        .bind(content)
        .bind(media_size)
        .bind(media_status)
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
         status = ?, content = ?, media_mime_type = ?, media_size = ?, media_status = ?,
         media_width = ?, media_height = ?, media_digest = ?, media_key = ?, media_waveform = ?,
         caption = ?, name = ?, thumb_image = ?, media_duration = ?
          WHERE message_id = ?"#,
        )
        .bind(update.status)
        .bind(&update.content)
        .bind(&update.media_mime_type)
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
        sqlx::query("UPDATE messages SET shared_user_id = ?, status = ? WHERE message_id = ?")
            .bind(user_id)
            .bind(status)
            .bind(message_id)
            .execute(&self.0)
            .await?;
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

    pub async fn update_media_status(
        &self,
        message_id: &str,
        status: MediaStatus,
    ) -> Result<(), Error> {
        sqlx::query("UPDATE messages SET media_status = ? WHERE message_id = ?")
            .bind(status)
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn complete_attachment_download(
        &self,
        message_id: &str,
        media_url: &str,
        media_size: i64,
        status: MediaStatus,
        content: &str,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE messages SET media_url = ?, media_size = ?, media_status = ?, content = ? \
             WHERE message_id = ?",
        )
        .bind(media_url)
        .bind(media_size)
        .bind(status)
        .bind(content)
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

            for chunk in message_ids.chunks(MARK_LIMIT - 1) {
                let mut query_builder: QueryBuilder<Sqlite> =
                    QueryBuilder::new("UPDATE messages SET quote_content = ");
                query_builder
                    .push_bind(&content)
                    .push(" WHERE message_id IN (");
                let mut separated = query_builder.separated(", ");
                for message_id in chunk {
                    separated.push_bind(message_id);
                }
                separated.push_unseparated(")");
                query_builder.build().execute(&self.0).await?;
            }
        }

        Ok(())
    }

    pub async fn mini_message_by_ids(&self, ids: &[String]) -> Result<Vec<MiniMessageItem>, Error> {
        let mut result = Vec::new();
        for chunk in ids.chunks(MARK_LIMIT) {
            let query_str = format!(
                "SELECT conversation_id, message_id FROM messages WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            result.extend(
                sqlx::query_as::<_, MiniMessageItem>(sqlx::AssertSqlSafe(query_str))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?,
            );
        }
        Ok(result)
    }

    pub async fn mark_message_read(&self, messages: &[String]) -> Result<(), Error> {
        let iter = messages.chunks(MARK_LIMIT - 3);
        for chunk in iter {
            let ids = chunk.iter().map(|m| m.as_str()).collect::<Vec<&str>>();
            let query = format!(
                "UPDATE messages SET status = ? WHERE message_id IN ({}) \
                 AND status != ? AND status != ?",
                expand_var(chunk.len())
            );
            let _ = sqlx::query(sqlx::AssertSqlSafe(query))
                .bind(MessageStatus::Read)
                .bind_list(&ids)
                .bind(MessageStatus::Failed)
                .bind(MessageStatus::Unknown)
                .execute(&self.0)
                .await?;
        }

        Ok(())
    }

    pub async fn find_sending_message(&self, message_id: &str) -> Result<Option<Message>, Error> {
        let message = sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE message_id = ? AND status = ? \
             AND content IS NOT NULL LIMIT 1",
        )
        .bind(message_id)
        .bind(MessageStatus::Sending)
        .fetch_optional(&self.0)
        .await?;
        Ok(message)
    }

    pub async fn update_message_category(
        &self,
        message_id: &str,
        category: &str,
    ) -> Result<u64, Error> {
        let result = sqlx::query("UPDATE messages SET category = ? WHERE message_id = ?")
            .bind(category)
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn update_message_status(
        &self,
        message_id: &str,
        status: MessageStatus,
    ) -> Result<u64, Error> {
        let result = sqlx::query("UPDATE messages SET status = ? WHERE message_id = ?")
            .bind(status)
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn advance_message_status(
        &self,
        message_id: &str,
        status: MessageStatus,
    ) -> Result<bool, Error> {
        let rank = match status {
            MessageStatus::Failed | MessageStatus::Unknown => return Ok(false),
            MessageStatus::Sending => 2,
            MessageStatus::Sent => 3,
            MessageStatus::Delivered => 4,
            MessageStatus::Read => 5,
        };
        let result = sqlx::query(
            "UPDATE messages SET status = ? WHERE message_id = ? \
             AND status != ? AND status != ? \
             AND CASE status \
                 WHEN 'SENDING' THEN 2 WHEN 'SENT' THEN 3 \
                 WHEN 'DELIVERED' THEN 4 WHEN 'READ' THEN 5 ELSE 1 END < ?",
        )
        .bind(status)
        .bind(message_id)
        .bind(MessageStatus::Failed)
        .bind(MessageStatus::Unknown)
        .bind(rank)
        .execute(&self.0)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        Ok(true)
    }

    pub async fn recall_message(
        &self,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<u64, Error> {
        let mut transaction = self.0.begin().await?;

        let result = sqlx::query(
            r#"UPDATE messages SET
               category = ?, content = NULL, media_url = NULL, media_mime_type = NULL,
               media_size = NULL, media_duration = '', media_width = NULL, media_height = NULL,
               media_hash = NULL, thumb_image = NULL, media_key = NULL, media_digest = NULL,
               media_status = 'CANCELED', action = NULL, participant_id = NULL,
               snapshot_id = NULL, hyperlink = NULL, name = NULL, album_id = NULL,
               sticker_id = NULL, shared_user_id = NULL, media_waveform = NULL,
               quote_message_id = NULL, quote_content = NULL, thumb_url = NULL, caption = NULL
               WHERE conversation_id = ? AND message_id = ?"#,
        )
        .bind(MESSAGE_RECALL)
        .bind(conversation_id)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "UPDATE messages SET content = NULL WHERE conversation_id = ? \
             AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
        )
        .bind(conversation_id)
        .bind(MESSAGE_PIN)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        sqlx::query("DELETE FROM pin_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM message_fts WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM message_mentions WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        self.update_message_quote_if_need(conversation_id, message_id)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_message(
        &self,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<u64, Error> {
        let mut transaction = self.0.begin().await?;

        sqlx::query(
            "UPDATE messages SET content = NULL WHERE conversation_id = ? \
             AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
        )
        .bind(conversation_id)
        .bind(MESSAGE_PIN)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("DELETE FROM pin_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM expired_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM message_mentions WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM message_fts WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;

        let result =
            sqlx::query("DELETE FROM messages WHERE conversation_id = ? AND message_id = ?")
                .bind(conversation_id)
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;

        let latest = sqlx::query_as::<_, (String, NaiveDateTime)>(
            "SELECT message_id, created_at FROM messages WHERE conversation_id = ? \
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let (last_message_id, last_message_created_at) = latest
            .map(|(message_id, created_at)| {
                (
                    Some(message_id),
                    Some(created_at.and_utc().timestamp_millis()),
                )
            })
            .unwrap_or_default();

        sqlx::query(
            "UPDATE conversations SET last_message_id = ?, last_message_created_at = ? \
             WHERE conversation_id = ?",
        )
        .bind(last_message_id)
        .bind(last_message_created_at)
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(result.rows_affected())
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
    use chrono::Utc;

    use sdk::message_category::{MESSAGE_PIN, PLAIN_TEXT};

    use super::*;
    use crate::db::mixin::MixinDatabase;

    async fn test_database() -> (tempfile::TempDir, MixinDatabase) {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        (directory, database)
    }

    fn message(message_id: &str) -> Message {
        Message {
            message_id: message_id.to_string(),
            conversation_id: "conversation".to_string(),
            user_id: "sender".to_string(),
            category: PLAIN_TEXT.to_string(),
            content: Some("hello".to_string()),
            status: MessageStatus::Sending,
            created_at: Utc::now().naive_utc(),
            ..Message::default()
        }
    }

    #[tokio::test]
    async fn updates_attachment_mime_type_and_propagates_errors() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        dao.insert_message(&message("attachment")).await.unwrap();

        dao.update_attachment_message(
            "attachment",
            &AttachmentMessageUpdate {
                status: MessageStatus::Sent,
                content: "content".to_string(),
                media_mime_type: "image/png".to_string(),
                media_size: 42,
                media_status: MediaStatus::Done,
                media_width: Some(1),
                media_height: Some(2),
                media_digest: None,
                media_key: None,
                media_waveform: None,
                caption: None,
                name: None,
                thumb_image: None,
                media_duration: None,
            },
        )
        .await
        .unwrap();

        let updated = dao
            .find_message_by_id(&"attachment".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.media_mime_type.as_deref(), Some("image/png"));
        assert_eq!(updated.media_status, MediaStatus::Done);
    }

    #[tokio::test]
    async fn updates_quote_content_with_valid_dynamic_sql() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        sqlx::query("INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)")
            .bind("sender")
            .bind("7000")
            .bind("Sender")
            .execute(&dao.0)
            .await
            .unwrap();

        dao.insert_message(&message("quoted")).await.unwrap();
        let mut reply = message("reply");
        reply.quote_message_id = Some("quoted".to_string());
        reply.quote_content = None;
        dao.insert_message(&reply).await.unwrap();

        dao.update_message_quote_if_need("conversation", "quoted")
            .await
            .unwrap();
        let reply = dao
            .find_message_by_id(&"reply".to_string())
            .await
            .unwrap()
            .unwrap();
        assert!(reply.quote_content.unwrap().contains("quoted"));
    }

    #[tokio::test]
    async fn reads_and_updates_sending_message() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        dao.insert_message(&message("sending")).await.unwrap();

        assert!(dao.find_sending_message("sending").await.unwrap().is_some());
        assert_eq!(
            dao.update_message_category("sending", "SIGNAL_TEXT")
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            dao.update_message_status("sending", MessageStatus::Sent)
                .await
                .unwrap(),
            1
        );
        assert!(dao.find_sending_message("sending").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn recalls_message_and_clears_pin_content() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao.clone();
        sqlx::query("INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)")
            .bind("sender")
            .bind("7000")
            .bind("Sender")
            .execute(&dao.0)
            .await
            .unwrap();
        let mut original = message("original");
        original.media_url = Some("secret-file".to_string());
        original.media_mime_type = Some("image/png".to_string());
        dao.insert_message(&original).await.unwrap();
        database
            .message_fts_dao
            .upsert("original", "conversation", "secret content")
            .await
            .unwrap();

        let mut pin = message("pin");
        pin.category = MESSAGE_PIN.to_string();
        pin.quote_message_id = Some("original".to_string());
        dao.insert_message(&pin).await.unwrap();
        let mut reply = message("reply");
        reply.quote_message_id = Some("original".to_string());
        dao.insert_message(&reply).await.unwrap();
        sqlx::query(
            "INSERT INTO pin_messages (message_id, conversation_id, created_at) VALUES (?, ?, ?)",
        )
        .bind("original")
        .bind("conversation")
        .bind(Utc::now())
        .execute(&dao.0)
        .await
        .unwrap();

        assert_eq!(
            dao.recall_message("conversation", "original")
                .await
                .unwrap(),
            1
        );
        let original = dao
            .find_message_by_id(&"original".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(original.category, MESSAGE_RECALL);
        assert_eq!(original.content, None);
        assert_eq!(original.media_url, None);
        assert_eq!(original.media_mime_type, None);

        let pin = dao
            .find_message_by_id(&"pin".to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(pin.content, None);
        let reply = dao
            .find_message_by_id(&"reply".to_string())
            .await
            .unwrap()
            .unwrap();
        let quote: serde_json::Value =
            serde_json::from_str(reply.quote_content.as_deref().unwrap()).unwrap();
        assert_eq!(quote["type"], MESSAGE_RECALL);
        let pin_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pin_messages WHERE message_id = 'original'")
                .fetch_one(&dao.0)
                .await
                .unwrap();
        assert_eq!(pin_count, 0);
        assert!(database
            .message_fts_dao
            .search("secret", None, 10)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn marks_message_read_with_parenthesized_placeholders() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        dao.insert_message(&message("one")).await.unwrap();
        dao.insert_message(&message("two")).await.unwrap();

        dao.mark_message_read(&["one".to_string(), "two".to_string()])
            .await
            .unwrap();
        for id in ["one", "two"] {
            let message = dao
                .find_message_by_id(&id.to_string())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(message.status, MessageStatus::Read);
        }
    }

    #[tokio::test]
    async fn read_receipts_do_not_revive_failed_or_unknown_messages() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        let mut failed = message("failed");
        failed.status = MessageStatus::Failed;
        let mut unknown = message("unknown");
        unknown.status = MessageStatus::Unknown;
        dao.insert_message(&failed).await.unwrap();
        dao.insert_message(&unknown).await.unwrap();

        let ids = vec!["failed".to_string(), "unknown".to_string()];
        dao.mark_message_read(&ids).await.unwrap();
        assert!(!dao
            .advance_message_status("failed", MessageStatus::Read)
            .await
            .unwrap());
        assert!(!dao
            .advance_message_status("unknown", MessageStatus::Delivered)
            .await
            .unwrap());

        for (id, status) in [
            ("failed", MessageStatus::Failed),
            ("unknown", MessageStatus::Unknown),
        ] {
            assert_eq!(
                dao.find_message_by_id(&id.to_string())
                    .await
                    .unwrap()
                    .unwrap()
                    .status,
                status
            );
        }
    }

    #[tokio::test]
    async fn message_status_only_moves_forward() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        dao.insert_message(&message("status")).await.unwrap();

        assert!(dao
            .advance_message_status("status", MessageStatus::Read)
            .await
            .unwrap());
        assert!(!dao
            .advance_message_status("status", MessageStatus::Delivered)
            .await
            .unwrap());
        assert_eq!(
            dao.find_message_by_id(&"status".to_string())
                .await
                .unwrap()
                .unwrap()
                .status,
            MessageStatus::Read
        );
    }
}
