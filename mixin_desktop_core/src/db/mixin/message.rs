use std::collections::{HashMap, HashSet};

use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, QueryBuilder, Sqlite};

use sdk::blaze_message::{MessageStatus, CREATE_MESSAGE};
use sdk::message_category::{MESSAGE_PIN, MESSAGE_RECALL};
use sdk::ACKNOWLEDGE_MESSAGE_RECEIPTS;

use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::job::Job;
use crate::db::mixin::transcript_message::{TranscriptMessage, TranscriptMessageDao};
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageListItem {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub sender_name: String,
    pub sender_identity_number: String,
    pub sender_avatar_url: String,
    pub sender_is_verified: bool,
    pub sender_relationship: String,
    pub sender_app_id: Option<String>,
    pub sender_is_scam: bool,
    pub sender_is_bot: bool,
    pub category: String,
    pub content: Option<String>,
    pub status: MessageStatus,
    pub created_at: NaiveDateTime,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_duration: String,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub thumb_image: Option<String>,
    pub media_status: MediaStatus,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ImageMessageItem {
    pub message_id: String,
    pub created_at: NaiveDateTime,
    pub media_url: String,
    pub media_name: Option<String>,
    pub can_forward: bool,
}

impl MessageListItem {
    pub fn created_at_micros(&self) -> i64 {
        self.created_at.and_utc().timestamp_micros()
    }
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
    pub async fn unread_message_ids(
        &self,
        conversation_id: &str,
        current_user_id: &str,
    ) -> Result<Vec<String>, Error> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND user_id != ? \
             AND status IN ('SENT', 'DELIVERED') ORDER BY created_at ASC, rowid ASC",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn mark_conversation_read(
        &self,
        conversation_id: &str,
        current_user_id: &str,
    ) -> Result<bool, Error> {
        let mut transaction = self.0.begin().await?;
        let message_ids = sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND user_id != ? \
             AND status IN ('SENT', 'DELIVERED') ORDER BY created_at ASC, rowid ASC",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_all(&mut *transaction)
        .await?;
        if message_ids.is_empty() {
            transaction.commit().await?;
            return Ok(false);
        }

        for chunk in message_ids.chunks(MARK_LIMIT) {
            let sql = format!(
                "UPDATE messages SET status = 'READ' WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind_list(chunk)
                .execute(&mut *transaction)
                .await?;
        }

        let now = Utc::now().timestamp();
        for chunk in message_ids.chunks(MARK_LIMIT - 1) {
            let sql = format!(
                "UPDATE expired_messages SET expire_at = CAST((?1 + expire_in) AS INTEGER) \
                 WHERE (expire_at > (?1 + expire_in) OR expire_at IS NULL) \
                 AND message_id IN ({})",
                crate::db::mixin::util::expand_var_with_index(2, chunk.len())
            );
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .bind(now)
                .bind_list(chunk)
                .execute(&mut *transaction)
                .await?;
        }

        let mut expire_at_by_id = HashMap::new();
        for chunk in message_ids.chunks(MARK_LIMIT) {
            let sql = format!(
                "SELECT message_id, expire_at FROM expired_messages WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            expire_at_by_id.extend(
                sqlx::query_as::<_, (String, Option<i64>)>(sqlx::AssertSqlSafe(sql))
                    .bind_list(chunk)
                    .fetch_all(&mut *transaction)
                    .await?,
            );
        }

        let last_read_message_id = sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND status = 'READ' \
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let unseen: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM messages WHERE conversation_id = ? \
             AND status IN ('SENT', 'DELIVERED') AND user_id != ?",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE conversations SET last_read_message_id = ?, unseen_message_count = ? \
             WHERE conversation_id = ?",
        )
        .bind(last_read_message_id)
        .bind(unseen)
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;

        let jobs = [ACKNOWLEDGE_MESSAGE_RECEIPTS, CREATE_MESSAGE]
            .into_iter()
            .flat_map(|action| {
                message_ids.iter().map(|message_id| {
                    Job::create_ack_job(
                        action,
                        message_id,
                        "READ",
                        expire_at_by_id.get(message_id).copied().flatten(),
                    )
                })
            })
            .collect::<Vec<_>>();
        const JOB_COLUMN_COUNT: usize = 10;
        for jobs in jobs.chunks(MARK_LIMIT / JOB_COLUMN_COUNT) {
            let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
                "INSERT OR REPLACE INTO jobs \
                 (job_id, action, created_at, order_id, priority, user_id, conversation_id, \
                 resend_message_id, run_count, blaze_message) ",
            );
            query_builder.push_values(jobs, |mut builder, job| {
                builder
                    .push_bind(&job.job_id)
                    .push_bind(&job.action)
                    .push_bind(job.created_at)
                    .push_bind(job.order_id)
                    .push_bind(job.priority)
                    .push_bind(job.user_id.as_ref())
                    .push_bind(job.conversation_id.as_ref())
                    .push_bind(job.resend_message_id.as_ref())
                    .push_bind(job.run_count)
                    .push_bind(&job.blaze_message);
            });
            query_builder.build().execute(&mut *transaction).await?;
        }

        transaction.commit().await?;
        Ok(true)
    }

    pub async fn list_items(
        &self,
        conversation_id: &str,
        before_created_at: Option<NaiveDateTime>,
        before_message_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        let result = sqlx::query_as::<_, MessageListItem>(
            r#"
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       message.category,
       message.content,
       message.status,
       message.created_at,
       message.media_url,
       message.media_mime_type,
       message.media_size,
       message.media_duration,
       message.media_width,
       message.media_height,
       message.thumb_image,
       message.media_status,
       message.quote_message_id,
       message.quote_content,
       message.caption,
       message.action,
       message.participant_id,
       participant.full_name AS participant_full_name,
       message.snapshot_id,
       COALESCE(snapshot.type, safe_snapshot.type) AS snapshot_type,
       COALESCE(snapshot.amount, safe_snapshot.amount) AS snapshot_amount,
       COALESCE(snapshot.memo, safe_snapshot.memo) AS snapshot_memo,
       COALESCE(snapshot.asset_id, safe_snapshot.asset_id) AS snapshot_asset_id,
       COALESCE(asset.symbol, token.symbol) AS snapshot_asset_symbol,
       COALESCE(asset.icon_url, token.icon_url) AS snapshot_asset_icon_url,
       chain.icon_url AS snapshot_chain_icon_url,
       COALESCE(snapshot.opponent_id, safe_snapshot.opponent_id) AS snapshot_opponent_id,
       COALESCE(snapshot.transaction_hash, safe_snapshot.transaction_hash) AS snapshot_transaction_hash,
       CAST(COALESCE(snapshot.created_at, safe_snapshot.created_at) AS TEXT) AS snapshot_created_at,
       safe_snapshot.inscription_hash,
       inscription_item.collection_hash AS inscription_collection_hash,
       inscription_item.sequence AS inscription_sequence,
       inscription_item.content_type AS inscription_content_type,
       inscription_item.content_url AS inscription_content_url,
       inscription_collection.name AS inscription_name,
       inscription_collection.icon_url AS inscription_icon_url,
       message.hyperlink,
       message.name AS media_name,
       message.album_id,
       message.sticker_id,
       message.shared_user_id,
       message.media_waveform,
       message.thumb_url,
       conversation.owner_id AS conversation_owner_id,
       conversation.category AS conversation_category,
       shared_user.full_name AS shared_user_full_name,
       shared_user.identity_number AS shared_user_identity_number,
       shared_user.avatar_url AS shared_user_avatar_url,
       COALESCE(shared_user.is_verified, FALSE) AS shared_user_is_verified,
       shared_user.app_id AS shared_user_app_id,
       sticker.asset_url AS sticker_asset_url,
       sticker.asset_width AS sticker_asset_width,
       sticker.asset_height AS sticker_asset_height,
       sticker.name AS sticker_asset_name,
       sticker.asset_type AS sticker_asset_type,
       mention.has_read AS mention_read,
       CASE WHEN pin.message_id IS NOT NULL THEN TRUE ELSE FALSE END AS pinned,
       CASE
           WHEN message.category = 'SYSTEM_CONVERSATION' AND message.action = 'EXPIRE'
               THEN CAST(message.content AS INTEGER)
           ELSE expired.expire_in
       END AS expire_in
FROM messages message
LEFT JOIN users sender ON sender.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN snapshots snapshot ON snapshot.snapshot_id = message.snapshot_id
LEFT JOIN safe_snapshots safe_snapshot ON safe_snapshot.snapshot_id = message.snapshot_id
LEFT JOIN assets asset ON asset.asset_id = snapshot.asset_id
LEFT JOIN tokens token ON token.asset_id = safe_snapshot.asset_id
LEFT JOIN chains chain ON chain.chain_id = COALESCE(asset.chain_id, token.chain_id)
LEFT JOIN inscription_items inscription_item
       ON inscription_item.inscription_hash = safe_snapshot.inscription_hash
LEFT JOIN inscription_collections inscription_collection
       ON inscription_collection.collection_hash = inscription_item.collection_hash
LEFT JOIN message_mentions mention ON mention.message_id = message.message_id
LEFT JOIN pin_messages pin ON pin.message_id = message.message_id
LEFT JOIN expired_messages expired ON expired.message_id = message.message_id
WHERE message.conversation_id = ?1
  AND (
      ?2 IS NULL
      OR message.created_at < ?2
      OR (
          message.created_at = ?2
          AND message.message_id < ?3
      )
  )
ORDER BY message.created_at DESC, message.message_id DESC
LIMIT ?4
            "#,
        )
        .bind(conversation_id)
        .bind(before_created_at)
        .bind(before_message_id.unwrap_or_default())
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn list_pinned_items(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageListItem>, Error> {
        Ok(sqlx::query_as::<_, MessageListItem>(
            r#"
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       message.category,
       message.content,
       message.status,
       message.created_at,
       message.media_url,
       message.media_mime_type,
       message.media_size,
       message.media_duration,
       message.media_width,
       message.media_height,
       message.thumb_image,
       message.media_status,
       message.quote_message_id,
       message.quote_content,
       message.caption,
       message.action,
       message.participant_id,
       participant.full_name AS participant_full_name,
       message.snapshot_id,
       COALESCE(snapshot.type, safe_snapshot.type) AS snapshot_type,
       COALESCE(snapshot.amount, safe_snapshot.amount) AS snapshot_amount,
       COALESCE(snapshot.memo, safe_snapshot.memo) AS snapshot_memo,
       COALESCE(snapshot.asset_id, safe_snapshot.asset_id) AS snapshot_asset_id,
       COALESCE(asset.symbol, token.symbol) AS snapshot_asset_symbol,
       COALESCE(asset.icon_url, token.icon_url) AS snapshot_asset_icon_url,
       chain.icon_url AS snapshot_chain_icon_url,
       COALESCE(snapshot.opponent_id, safe_snapshot.opponent_id) AS snapshot_opponent_id,
       COALESCE(snapshot.transaction_hash, safe_snapshot.transaction_hash) AS snapshot_transaction_hash,
       CAST(COALESCE(snapshot.created_at, safe_snapshot.created_at) AS TEXT) AS snapshot_created_at,
       safe_snapshot.inscription_hash,
       inscription_item.collection_hash AS inscription_collection_hash,
       inscription_item.sequence AS inscription_sequence,
       inscription_item.content_type AS inscription_content_type,
       inscription_item.content_url AS inscription_content_url,
       inscription_collection.name AS inscription_name,
       inscription_collection.icon_url AS inscription_icon_url,
       message.hyperlink,
       message.name AS media_name,
       message.album_id,
       message.sticker_id,
       message.shared_user_id,
       message.media_waveform,
       message.thumb_url,
       conversation.owner_id AS conversation_owner_id,
       conversation.category AS conversation_category,
       shared_user.full_name AS shared_user_full_name,
       shared_user.identity_number AS shared_user_identity_number,
       shared_user.avatar_url AS shared_user_avatar_url,
       COALESCE(shared_user.is_verified, FALSE) AS shared_user_is_verified,
       shared_user.app_id AS shared_user_app_id,
       sticker.asset_url AS sticker_asset_url,
       sticker.asset_width AS sticker_asset_width,
       sticker.asset_height AS sticker_asset_height,
       sticker.name AS sticker_asset_name,
       sticker.asset_type AS sticker_asset_type,
       mention.has_read AS mention_read,
       TRUE AS pinned,
       CASE
           WHEN message.category = 'SYSTEM_CONVERSATION' AND message.action = 'EXPIRE'
               THEN CAST(message.content AS INTEGER)
           ELSE expired.expire_in
       END AS expire_in
FROM pin_messages selected_pin
INNER JOIN messages message ON message.message_id = selected_pin.message_id
LEFT JOIN users sender ON sender.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN snapshots snapshot ON snapshot.snapshot_id = message.snapshot_id
LEFT JOIN safe_snapshots safe_snapshot ON safe_snapshot.snapshot_id = message.snapshot_id
LEFT JOIN assets asset ON asset.asset_id = snapshot.asset_id
LEFT JOIN tokens token ON token.asset_id = safe_snapshot.asset_id
LEFT JOIN chains chain ON chain.chain_id = COALESCE(asset.chain_id, token.chain_id)
LEFT JOIN inscription_items inscription_item
       ON inscription_item.inscription_hash = safe_snapshot.inscription_hash
LEFT JOIN inscription_collections inscription_collection
       ON inscription_collection.collection_hash = inscription_item.collection_hash
LEFT JOIN message_mentions mention ON mention.message_id = message.message_id
LEFT JOIN expired_messages expired ON expired.message_id = message.message_id
WHERE selected_pin.conversation_id = ?1
  AND message.conversation_id = ?1
ORDER BY selected_pin.created_at DESC, message.message_id DESC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn list_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        let result = sqlx::query_as::<_, MessageListItem>(
            r#"
WITH target AS (
    SELECT created_at, message_id
    FROM messages
    WHERE conversation_id = ?1 AND message_id = ?2
), message_window AS (
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate, target
        WHERE candidate.conversation_id = ?1
          AND (
              candidate.created_at < target.created_at
              OR (
                  candidate.created_at = target.created_at
                  AND candidate.message_id < target.message_id
              )
          )
        ORDER BY candidate.created_at DESC, candidate.message_id DESC
        LIMIT ?3
    )
    UNION ALL
    SELECT message_id FROM target
    UNION ALL
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate, target
        WHERE candidate.conversation_id = ?1
          AND (
              candidate.created_at > target.created_at
              OR (
                  candidate.created_at = target.created_at
                  AND candidate.message_id > target.message_id
              )
          )
        ORDER BY candidate.created_at ASC, candidate.message_id ASC
        LIMIT ?4
    )
)
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       message.category,
       message.content,
       message.status,
       message.created_at,
       message.media_url,
       message.media_mime_type,
       message.media_size,
       message.media_duration,
       message.media_width,
       message.media_height,
       message.thumb_image,
       message.media_status,
       message.quote_message_id,
       message.quote_content,
       message.caption,
       message.action,
       message.participant_id,
       participant.full_name AS participant_full_name,
       message.snapshot_id,
       COALESCE(snapshot.type, safe_snapshot.type) AS snapshot_type,
       COALESCE(snapshot.amount, safe_snapshot.amount) AS snapshot_amount,
       COALESCE(snapshot.memo, safe_snapshot.memo) AS snapshot_memo,
       COALESCE(snapshot.asset_id, safe_snapshot.asset_id) AS snapshot_asset_id,
       COALESCE(asset.symbol, token.symbol) AS snapshot_asset_symbol,
       COALESCE(asset.icon_url, token.icon_url) AS snapshot_asset_icon_url,
       chain.icon_url AS snapshot_chain_icon_url,
       COALESCE(snapshot.opponent_id, safe_snapshot.opponent_id) AS snapshot_opponent_id,
       COALESCE(snapshot.transaction_hash, safe_snapshot.transaction_hash) AS snapshot_transaction_hash,
       CAST(COALESCE(snapshot.created_at, safe_snapshot.created_at) AS TEXT) AS snapshot_created_at,
       safe_snapshot.inscription_hash,
       inscription_item.collection_hash AS inscription_collection_hash,
       inscription_item.sequence AS inscription_sequence,
       inscription_item.content_type AS inscription_content_type,
       inscription_item.content_url AS inscription_content_url,
       inscription_collection.name AS inscription_name,
       inscription_collection.icon_url AS inscription_icon_url,
       message.hyperlink,
       message.name AS media_name,
       message.album_id,
       message.sticker_id,
       message.shared_user_id,
       message.media_waveform,
       message.thumb_url,
       conversation.owner_id AS conversation_owner_id,
       conversation.category AS conversation_category,
       shared_user.full_name AS shared_user_full_name,
       shared_user.identity_number AS shared_user_identity_number,
       shared_user.avatar_url AS shared_user_avatar_url,
       COALESCE(shared_user.is_verified, FALSE) AS shared_user_is_verified,
       shared_user.app_id AS shared_user_app_id,
       sticker.asset_url AS sticker_asset_url,
       sticker.asset_width AS sticker_asset_width,
       sticker.asset_height AS sticker_asset_height,
       sticker.name AS sticker_asset_name,
       sticker.asset_type AS sticker_asset_type,
       mention.has_read AS mention_read,
       CASE WHEN pin.message_id IS NOT NULL THEN TRUE ELSE FALSE END AS pinned,
       CASE
           WHEN message.category = 'SYSTEM_CONVERSATION' AND message.action = 'EXPIRE'
               THEN CAST(message.content AS INTEGER)
           ELSE expired.expire_in
       END AS expire_in
FROM message_window
INNER JOIN messages message ON message.message_id = message_window.message_id
LEFT JOIN users sender ON sender.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN snapshots snapshot ON snapshot.snapshot_id = message.snapshot_id
LEFT JOIN safe_snapshots safe_snapshot ON safe_snapshot.snapshot_id = message.snapshot_id
LEFT JOIN assets asset ON asset.asset_id = snapshot.asset_id
LEFT JOIN tokens token ON token.asset_id = safe_snapshot.asset_id
LEFT JOIN chains chain ON chain.chain_id = COALESCE(asset.chain_id, token.chain_id)
LEFT JOIN inscription_items inscription_item
       ON inscription_item.inscription_hash = safe_snapshot.inscription_hash
LEFT JOIN inscription_collections inscription_collection
       ON inscription_collection.collection_hash = inscription_item.collection_hash
LEFT JOIN message_mentions mention ON mention.message_id = message.message_id
LEFT JOIN pin_messages pin ON pin.message_id = message.message_id
LEFT JOIN expired_messages expired ON expired.message_id = message.message_id
ORDER BY message.created_at ASC, message.message_id ASC
            "#,
        )
        .bind(conversation_id)
        .bind(target_message_id)
        .bind(before.clamp(0, 100))
        .bind(after.clamp(0, 100))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn list_image_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<ImageMessageItem>, Error> {
        Ok(sqlx::query_as::<_, ImageMessageItem>(
            r#"
WITH target AS (
    SELECT created_at, message_id
    FROM messages
    WHERE conversation_id = ?1
      AND message_id = ?2
      AND substr(category, -6) = '_IMAGE'
      AND COALESCE(media_url, '') != ''
), image_window AS (
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate, target
        WHERE candidate.conversation_id = ?1
          AND substr(candidate.category, -6) = '_IMAGE'
          AND COALESCE(candidate.media_url, '') != ''
          AND (
              candidate.created_at < target.created_at
              OR (candidate.created_at = target.created_at AND candidate.message_id < target.message_id)
          )
        ORDER BY candidate.created_at DESC, candidate.message_id DESC
        LIMIT ?3
    )
    UNION ALL
    SELECT message_id FROM target
    UNION ALL
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate, target
        WHERE candidate.conversation_id = ?1
          AND substr(candidate.category, -6) = '_IMAGE'
          AND COALESCE(candidate.media_url, '') != ''
          AND (
              candidate.created_at > target.created_at
              OR (candidate.created_at = target.created_at AND candidate.message_id > target.message_id)
          )
        ORDER BY candidate.created_at ASC, candidate.message_id ASC
        LIMIT ?4
    )
)
SELECT message.message_id,
       message.created_at,
       message.media_url,
       message.name AS media_name,
       CASE
           WHEN message.status IN ('SENT', 'DELIVERED', 'READ')
            AND message.media_status IN ('DONE', 'READ')
            AND json_valid(message.content)
            AND COALESCE(json_extract(message.content, '$.shareable'), TRUE) != FALSE
           THEN TRUE ELSE FALSE
       END AS can_forward
FROM image_window
INNER JOIN messages message ON message.message_id = image_window.message_id
ORDER BY message.created_at ASC, message.message_id ASC
            "#,
        )
        .bind(conversation_id)
        .bind(target_message_id)
        .bind(before.clamp(0, 100))
        .bind(after.clamp(0, 100))
        .fetch_all(&self.0)
        .await?)
    }

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

    pub async fn find_messages_by_ids(
        &self,
        message_ids: &[String],
    ) -> Result<Vec<Message>, Error> {
        let mut messages = Vec::new();
        for chunk in message_ids.chunks(MARK_LIMIT) {
            let query = format!(
                "SELECT * FROM messages WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            messages.extend(
                sqlx::query_as::<_, Message>(sqlx::AssertSqlSafe(query))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?,
            );
        }
        Ok(messages)
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
        insert_message_with(&self.0, message).await?;
        Ok(())
    }

    pub async fn insert_outgoing_message(&self, message: &Message, job: &Job) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        Self::insert_outgoing_message_with(&mut transaction, message, job).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_outgoing_message_with_transcripts(
        &self,
        message: &Message,
        job: &Job,
        transcripts: &[TranscriptMessage],
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        TranscriptMessageDao::insert_all_with(&mut transaction, transcripts).await?;
        Self::insert_outgoing_message_with(&mut transaction, message, job).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn insert_outgoing_message_with(
        transaction: &mut sqlx::Transaction<'_, Sqlite>,
        message: &Message,
        job: &Job,
    ) -> Result<(), Error> {
        insert_message_with(&mut **transaction, message).await?;
        sqlx::query(
            "UPDATE conversations SET last_message_id = ?, last_message_created_at = ?, \
             draft = '' WHERE conversation_id = ?",
        )
        .bind(&message.message_id)
        .bind(message.created_at.and_utc().timestamp_millis())
        .bind(&message.conversation_id)
        .execute(&mut **transaction)
        .await?;
        Self::insert_job_with(transaction, job).await?;
        Ok(())
    }

    async fn insert_job_with(
        transaction: &mut sqlx::Transaction<'_, Sqlite>,
        job: &Job,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO jobs (job_id, action, created_at, order_id, priority, user_id,
             conversation_id, resend_message_id, run_count, blaze_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&job.job_id)
        .bind(&job.action)
        .bind(job.created_at)
        .bind(job.order_id)
        .bind(job.priority)
        .bind(job.user_id.as_ref())
        .bind(job.conversation_id.as_ref())
        .bind(job.resend_message_id.as_ref())
        .bind(job.run_count)
        .bind(&job.blaze_message)
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn complete_sending_job(
        &self,
        message_id: &str,
        content: Option<&str>,
        status: MessageStatus,
        expire_in: i64,
        job_id: &str,
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        if let Some(content) = content {
            sqlx::query("UPDATE messages SET content = ? WHERE message_id = ?")
                .bind(content)
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
        }
        let target = match status {
            MessageStatus::Sent => "SENT",
            MessageStatus::Failed => "FAILED",
            _ => {
                return Err(anyhow::anyhow!("invalid sending completion status: {status:?}").into())
            }
        };
        sqlx::query("UPDATE messages SET status = ? WHERE message_id = ? AND status = 'SENDING'")
            .bind(target)
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        if status == MessageStatus::Sent && expire_in > 0 {
            sqlx::query(
                "INSERT INTO expired_messages (message_id, expire_in, expire_at) VALUES (?, ?, ?) \
                 ON CONFLICT(message_id) DO UPDATE SET \
                 expire_in = excluded.expire_in, expire_at = excluded.expire_at",
            )
            .bind(message_id)
            .bind(expire_in)
            .bind(Utc::now().timestamp() + expire_in)
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query("DELETE FROM jobs WHERE job_id = ?")
            .bind(job_id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
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

    pub async fn complete_attachment_download_if_pending(
        &self,
        message_id: &str,
        media_url: &str,
        media_size: i64,
        status: MediaStatus,
        content: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE messages SET media_url = ?, media_size = ?, media_status = ?, content = ? \
             WHERE message_id = ? AND media_status = ?",
        )
        .bind(media_url)
        .bind(media_size)
        .bind(status)
        .bind(content)
        .bind(message_id)
        .bind(MediaStatus::Pending)
        .execute(&self.0)
        .await?;
        Ok(result.rows_affected() > 0)
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

    pub async fn delete_messages_batch(
        &self,
        conversation_id: &str,
        message_ids: &[String],
    ) -> Result<u64, Error> {
        if message_ids.iter().collect::<HashSet<_>>().len() != message_ids.len() {
            return Err(anyhow::anyhow!("delete message ids contain duplicates").into());
        }
        let mut transaction = self.0.begin().await?;
        for message_id in message_ids {
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM messages \
                 WHERE conversation_id = ? AND message_id = ?)",
            )
            .bind(conversation_id)
            .bind(message_id)
            .fetch_one(&mut *transaction)
            .await?;
            if !exists {
                return Err(
                    anyhow::anyhow!("message not found in conversation: {message_id}").into(),
                );
            }
        }
        let mut deleted = 0;
        for message_id in message_ids {
            sqlx::query(
                "UPDATE messages SET content = NULL WHERE conversation_id = ? \
                 AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
            )
            .bind(conversation_id)
            .bind(MESSAGE_PIN)
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
            for table in [
                "pin_messages",
                "expired_messages",
                "message_mentions",
                "message_fts",
            ] {
                let query = format!("DELETE FROM {table} WHERE message_id = ?");
                sqlx::query(sqlx::AssertSqlSafe(query))
                    .bind(message_id)
                    .execute(&mut *transaction)
                    .await?;
            }
            sqlx::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
            deleted +=
                sqlx::query("DELETE FROM messages WHERE conversation_id = ? AND message_id = ?")
                    .bind(conversation_id)
                    .bind(message_id)
                    .execute(&mut *transaction)
                    .await?
                    .rows_affected();
        }
        Self::update_conversation_last_message_with(&mut transaction, conversation_id).await?;
        transaction.commit().await?;
        Ok(deleted)
    }

    pub async fn clear_conversation(&self, conversation_id: &str) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        sqlx::query(
            "DELETE FROM transcript_messages WHERE transcript_id IN (\
             SELECT message_id FROM messages WHERE conversation_id = ?)",
        )
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "DELETE FROM expired_messages WHERE message_id IN (\
             SELECT message_id FROM messages WHERE conversation_id = ?)",
        )
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
        for table in [
            "pin_messages",
            "message_mentions",
            "message_fts",
            "messages",
        ] {
            let query = format!("DELETE FROM {table} WHERE conversation_id = ?");
            sqlx::query(sqlx::AssertSqlSafe(query))
                .bind(conversation_id)
                .execute(&mut *transaction)
                .await?;
        }
        Self::update_conversation_last_message_with(&mut transaction, conversation_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn recall_messages_with_jobs(
        &self,
        conversation_id: &str,
        message_ids: &[String],
        jobs: &[Job],
    ) -> Result<u64, Error> {
        if message_ids.len() != jobs.len() {
            return Err(anyhow::anyhow!("recall message and job counts do not match").into());
        }
        if message_ids.iter().collect::<HashSet<_>>().len() != message_ids.len() {
            return Err(anyhow::anyhow!("recall message ids contain duplicates").into());
        }
        let mut transaction = self.0.begin().await?;
        for job in jobs {
            Self::insert_job_with(&mut transaction, job).await?;
        }
        let mut recalled = 0;
        for message_id in message_ids {
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
            if result.rows_affected() != 1 {
                return Err(
                    anyhow::anyhow!("message not found in conversation: {message_id}").into(),
                );
            }
            recalled += 1;
            sqlx::query(
                "UPDATE messages SET content = NULL WHERE conversation_id = ? \
                 AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
            )
            .bind(conversation_id)
            .bind(MESSAGE_PIN)
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
            for table in ["pin_messages", "message_fts", "message_mentions"] {
                let query = format!("DELETE FROM {table} WHERE message_id = ?");
                sqlx::query(sqlx::AssertSqlSafe(query))
                    .bind(message_id)
                    .execute(&mut *transaction)
                    .await?;
            }
            sqlx::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
            Self::update_message_quote_with(&mut transaction, conversation_id, message_id).await?;
        }
        transaction.commit().await?;
        Ok(recalled)
    }

    pub async fn set_message_pinned_with_job(
        &self,
        conversation_id: &str,
        message_id: &str,
        pinned: bool,
        created_at: DateTime<Utc>,
        job: &Job,
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM messages \
             WHERE conversation_id = ? AND message_id = ?)",
        )
        .bind(conversation_id)
        .bind(message_id)
        .fetch_one(&mut *transaction)
        .await?;
        if !exists {
            return Err(anyhow::anyhow!("message not found in conversation: {message_id}").into());
        }
        Self::insert_job_with(&mut transaction, job).await?;
        if pinned {
            sqlx::query(
                "INSERT OR REPLACE INTO pin_messages (message_id, conversation_id, created_at) \
                 VALUES (?, ?, ?)",
            )
            .bind(message_id)
            .bind(conversation_id)
            .bind(created_at)
            .execute(&mut *transaction)
            .await?;
        } else {
            sqlx::query("DELETE FROM pin_messages WHERE message_id = ?")
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn update_message_quote_with(
        transaction: &mut sqlx::Transaction<'_, Sqlite>,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<(), Error> {
        let query = format!(
            "{} WHERE message.message_id = ?",
            QUOTE_MESSAGE_QUERY_PREFIX
        );
        let message = sqlx::query_as::<_, QuoteMessage>(sqlx::AssertSqlSafe(query))
            .bind(message_id)
            .fetch_optional(&mut **transaction)
            .await?;
        if let Some(message) = message {
            let content =
                serde_json::to_string(&message).with_context(|| "convert quote message to json")?;
            sqlx::query(
                "UPDATE messages SET quote_content = ? \
                 WHERE conversation_id = ? AND quote_message_id = ?",
            )
            .bind(content)
            .bind(conversation_id)
            .bind(message_id)
            .execute(&mut **transaction)
            .await?;
        }
        Ok(())
    }

    async fn update_conversation_last_message_with(
        transaction: &mut sqlx::Transaction<'_, Sqlite>,
        conversation_id: &str,
    ) -> Result<(), Error> {
        let latest = sqlx::query_as::<_, (String, NaiveDateTime)>(
            "SELECT message_id, created_at FROM messages WHERE conversation_id = ? \
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(&mut **transaction)
        .await?;
        let (message_id, created_at) = latest
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
        .bind(message_id)
        .bind(created_at)
        .bind(conversation_id)
        .execute(&mut **transaction)
        .await?;
        Ok(())
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

async fn insert_message_with<'e, E>(executor: E, message: &Message) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        r#"
INSERT OR REPLACE INTO messages (message_id, conversation_id, user_id, category, content,
media_url, media_mime_type, media_size, media_duration, media_width, media_height, media_hash,
thumb_image, media_key, media_digest, media_status, status, created_at, action, participant_id,
snapshot_id, hyperlink, name, album_id, sticker_id, shared_user_id, media_waveform, quote_message_id,
quote_content, thumb_url, caption)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
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
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use sdk::message_category::{MESSAGE_PIN, PLAIN_IMAGE, PLAIN_TEXT};
    use sdk::ConversationCategory;

    use super::*;
    use crate::db::mixin::conversation::{Conversation, ConversationStatus};
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
    async fn inserts_outgoing_message_projection_and_job_atomically() {
        let (_directory, database) = test_database().await;
        let now = Utc::now();
        database
            .conversation_dao
            .insert(&Conversation {
                conversation_id: "conversation".into(),
                owner_id: Some("other".into()),
                category: Some(ConversationCategory::Contact),
                name: String::new(),
                icon_url: String::new(),
                announcement: String::new(),
                code_url: String::new(),
                created_at: now,
                status: ConversationStatus::SUCCESS,
                mute_until: now,
                expire_in: 0,
            })
            .await
            .unwrap();
        let outgoing = Message {
            created_at: now.naive_utc(),
            ..message("outgoing")
        };
        let job = Job::create_sending_job(
            &outgoing.message_id,
            &outgoing.conversation_id,
            None,
            None,
            false,
            false,
            0,
        );

        database
            .message_dao
            .insert_outgoing_message(&outgoing, &job)
            .await
            .unwrap();

        assert!(database
            .message_dao
            .find_message_by_id(&outgoing.message_id)
            .await
            .unwrap()
            .is_some());
        let persisted_job: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
            .bind(&job.job_id)
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(persisted_job, 1);
        let last_message_id: Option<String> = sqlx::query_scalar(
            "SELECT last_message_id FROM conversations WHERE conversation_id = ?",
        )
        .bind(&outgoing.conversation_id)
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert_eq!(
            last_message_id.as_deref(),
            Some(outgoing.message_id.as_str())
        );
    }

    #[tokio::test]
    async fn rolls_back_transcripts_when_outgoing_message_fails() {
        let (_directory, database) = test_database().await;
        sqlx::query(
            "CREATE TRIGGER reject_outgoing_message BEFORE INSERT ON messages \
             WHEN NEW.message_id = 'outgoing' BEGIN SELECT RAISE(ABORT, 'rejected'); END",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let outgoing = message("outgoing");
        let job = Job::create_sending_job(
            &outgoing.message_id,
            &outgoing.conversation_id,
            None,
            None,
            false,
            false,
            0,
        );
        let transcript: TranscriptMessage = serde_json::from_value(serde_json::json!({
            "transcript_id": outgoing.message_id.clone(),
            "message_id": "source",
            "category": PLAIN_TEXT,
            "created_at": Utc::now().to_rfc3339(),
            "content": "hello"
        }))
        .unwrap();

        assert!(database
            .message_dao
            .insert_outgoing_message_with_transcripts(&outgoing, &job, &[transcript])
            .await
            .is_err());
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transcript_messages WHERE transcript_id = 'outgoing'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn rolls_back_batch_delete_and_fts_cleanup_on_failure() {
        let (_directory, database) = test_database().await;
        for message_id in ["first", "second"] {
            database
                .message_dao
                .insert_message(&message(message_id))
                .await
                .unwrap();
            database
                .message_fts_dao
                .upsert(message_id, "conversation", "hello")
                .await
                .unwrap();
        }
        sqlx::query(
            "CREATE TRIGGER reject_second_delete BEFORE DELETE ON messages \
             WHEN OLD.message_id = 'second' BEGIN SELECT RAISE(ABORT, 'rejected'); END",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();

        assert!(database
            .message_dao
            .delete_messages_batch("conversation", &["first".to_string(), "second".to_string()],)
            .await
            .is_err());
        let messages: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE message_id IN ('first', 'second')",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let fts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM message_fts WHERE message_id IN ('first', 'second')",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert_eq!((messages, fts), (2, 2));
    }

    #[tokio::test]
    async fn clears_only_the_selected_conversation_and_related_indexes() {
        let (_directory, database) = test_database().await;
        for (message_id, conversation_id) in [
            ("first", "conversation"),
            ("second", "conversation"),
            ("other", "other-conversation"),
        ] {
            let mut item = message(message_id);
            item.conversation_id = conversation_id.to_string();
            database.message_dao.insert_message(&item).await.unwrap();
            database
                .message_fts_dao
                .upsert(message_id, conversation_id, "hello")
                .await
                .unwrap();
        }
        sqlx::query(
            "INSERT INTO pin_messages (message_id, conversation_id, created_at) \
             VALUES ('first', 'conversation', CURRENT_TIMESTAMP)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO expired_messages (message_id, expire_in) \
             VALUES ('second', CURRENT_TIMESTAMP)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();

        database
            .message_dao
            .clear_conversation("conversation")
            .await
            .unwrap();

        let selected_messages: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let other_messages: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = 'other-conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let selected_fts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM message_fts WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let pin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pin_messages WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let expired_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM expired_messages WHERE message_id = 'second'")
                .fetch_one(&database.message_dao.0)
                .await
                .unwrap();
        assert_eq!(selected_messages, 0);
        assert_eq!(selected_fts, 0);
        assert_eq!(pin_count, 0);
        assert_eq!(expired_count, 0);
        assert_eq!(other_messages, 1);
    }

    #[tokio::test]
    async fn rolls_back_recall_jobs_and_messages_on_failure() {
        let (_directory, database) = test_database().await;
        for message_id in ["first", "second"] {
            database
                .message_dao
                .insert_message(&message(message_id))
                .await
                .unwrap();
        }
        sqlx::query(
            "CREATE TRIGGER reject_second_recall BEFORE UPDATE ON messages \
             WHEN OLD.message_id = 'second' AND NEW.category = 'MESSAGE_RECALL' \
             BEGIN SELECT RAISE(ABORT, 'rejected'); END",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let ids = ["first".to_string(), "second".to_string()];
        let jobs = ids
            .iter()
            .map(|message_id| Job::create_send_recall_job("conversation", message_id))
            .collect::<Vec<_>>();

        assert!(database
            .message_dao
            .recall_messages_with_jobs("conversation", &ids, &jobs)
            .await
            .is_err());
        let recalled: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE message_id IN ('first', 'second') \
             AND category = 'MESSAGE_RECALL'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!((recalled, jobs), (0, 0));
    }

    #[tokio::test]
    async fn rolls_back_pin_job_when_pin_mutation_fails() {
        let (_directory, database) = test_database().await;
        database
            .message_dao
            .insert_message(&message("message"))
            .await
            .unwrap();
        sqlx::query(
            "CREATE TRIGGER reject_pin BEFORE INSERT ON pin_messages \
             BEGIN SELECT RAISE(ABORT, 'rejected'); END",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let job = Job::create_send_pin_job("conversation", "payload");

        assert!(database
            .message_dao
            .set_message_pinned_with_job("conversation", "message", true, Utc::now(), &job,)
            .await
            .is_err());
        let jobs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(jobs, 0);
    }

    #[tokio::test]
    async fn completes_sending_message_expiration_and_job_atomically() {
        let (_directory, database) = test_database().await;
        let outgoing = message("outgoing");
        let job = Job::create_sending_job(
            &outgoing.message_id,
            &outgoing.conversation_id,
            None,
            None,
            false,
            false,
            60,
        );
        database
            .message_dao
            .insert_message(&outgoing)
            .await
            .unwrap();
        database.job_dao.insert_job(&job).await.unwrap();

        database
            .message_dao
            .complete_sending_job(
                &outgoing.message_id,
                Some("normalized"),
                MessageStatus::Sent,
                60,
                &job.job_id,
            )
            .await
            .unwrap();

        let persisted = database
            .message_dao
            .find_message_by_id(&outgoing.message_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.status, MessageStatus::Sent);
        assert_eq!(persisted.content.as_deref(), Some("normalized"));
        assert!(database
            .expired_message_dao
            .get_expired_message_by_id(&outgoing.message_id)
            .await
            .unwrap()
            .is_some());
        let job_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
            .bind(&job.job_id)
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(job_count, 0);
    }

    #[tokio::test]
    async fn sending_completion_never_downgrades_a_receipt_status() {
        let (_directory, database) = test_database().await;
        let outgoing = Message {
            status: MessageStatus::Read,
            ..message("already-read")
        };
        let job = Job::create_sending_job(
            &outgoing.message_id,
            &outgoing.conversation_id,
            None,
            None,
            false,
            false,
            0,
        );
        database
            .message_dao
            .insert_message(&outgoing)
            .await
            .unwrap();
        database.job_dao.insert_job(&job).await.unwrap();

        database
            .message_dao
            .complete_sending_job(
                &outgoing.message_id,
                Some("normalized"),
                MessageStatus::Sent,
                0,
                &job.job_id,
            )
            .await
            .unwrap();

        let persisted = database
            .message_dao
            .find_message_by_id(&outgoing.message_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.status, MessageStatus::Read);
        assert_eq!(persisted.content.as_deref(), Some("normalized"));
    }

    #[tokio::test]
    async fn lists_messages_with_a_stable_cursor() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        let base = Utc::now().naive_utc();
        for (message_id, offset) in [("oldest", 0), ("middle", 1), ("latest", 2)] {
            dao.insert_message(&Message {
                message_id: message_id.to_string(),
                created_at: base + chrono::Duration::seconds(offset),
                ..message(message_id)
            })
            .await
            .unwrap();
        }

        let latest = dao.list_items("conversation", None, None, 2).await.unwrap();
        assert_eq!(
            latest
                .iter()
                .map(|message| message.message_id.as_str())
                .collect::<Vec<_>>(),
            ["latest", "middle"]
        );

        let cursor = latest.last().unwrap();
        let cursor_created_at = cursor.created_at;
        let cursor_message_id = cursor.message_id.clone();
        sqlx::query("DELETE FROM messages WHERE message_id = ?")
            .bind(&cursor_message_id)
            .execute(&dao.0)
            .await
            .unwrap();
        let older = dao
            .list_items(
                "conversation",
                Some(cursor_created_at),
                Some(&cursor_message_id),
                2,
            )
            .await
            .unwrap();
        assert_eq!(
            older
                .iter()
                .map(|message| message.message_id.as_str())
                .collect::<Vec<_>>(),
            ["oldest"]
        );
    }

    #[tokio::test]
    async fn lists_pinned_messages_in_pin_order_with_full_projection() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        sqlx::query(
            "INSERT INTO users (user_id, identity_number, full_name, avatar_url, is_verified) \
             VALUES (?, ?, ?, ?, ?), (?, ?, ?, ?, ?)",
        )
        .bind("sender")
        .bind("7000")
        .bind("Sender")
        .bind("sender.png")
        .bind(true)
        .bind("shared")
        .bind("7001")
        .bind("Shared")
        .bind("shared.png")
        .bind(false)
        .execute(&dao.0)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO stickers \
             (sticker_id, name, asset_url, asset_type, asset_width, asset_height, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("sticker")
        .bind("Wave")
        .bind("sticker.webp")
        .bind("webp")
        .bind(128)
        .bind(96)
        .bind(Utc::now())
        .execute(&dao.0)
        .await
        .unwrap();

        for id in ["alpha", "beta", "latest"] {
            let mut item = message(id);
            if id == "latest" {
                item.shared_user_id = Some("shared".to_string());
                item.sticker_id = Some("sticker".to_string());
            }
            dao.insert_message(&item).await.unwrap();
        }
        let base = Utc::now();
        for (id, created_at) in [
            ("alpha", base),
            ("beta", base),
            ("latest", base + chrono::Duration::seconds(1)),
        ] {
            sqlx::query(
                "INSERT INTO pin_messages (message_id, conversation_id, created_at) \
                 VALUES (?, ?, ?)",
            )
            .bind(id)
            .bind("conversation")
            .bind(created_at)
            .execute(&dao.0)
            .await
            .unwrap();
        }

        let items = dao.list_pinned_items("conversation").await.unwrap();
        assert_eq!(
            items
                .iter()
                .map(|message| message.message_id.as_str())
                .collect::<Vec<_>>(),
            ["latest", "beta", "alpha"]
        );
        assert!(items.iter().all(|message| message.pinned));
        assert_eq!(items[0].sender_name, "Sender");
        assert_eq!(items[0].sender_identity_number, "7000");
        assert!(items[0].sender_is_verified);
        assert_eq!(items[0].shared_user_full_name.as_deref(), Some("Shared"));
        assert_eq!(items[0].sticker_asset_url.as_deref(), Some("sticker.webp"));
        assert_eq!(items[0].sticker_asset_width, Some(128));
    }

    #[tokio::test]
    async fn lists_image_messages_around_cursor_in_stable_order() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        let created_at = Utc::now().naive_utc();
        for id in ["alpha", "beta", "gamma"] {
            dao.insert_message(&Message {
                category: PLAIN_IMAGE.to_string(),
                content: Some(
                    r#"{"attachment_id":"attachment","message_id":"source"}"#.to_string(),
                ),
                media_url: Some(format!("/{id}.jpg")),
                name: Some(format!("{id}.jpg")),
                status: MessageStatus::Sent,
                media_status: MediaStatus::Done,
                created_at,
                ..message(id)
            })
            .await
            .unwrap();
        }
        dao.insert_message(&Message {
            category: PLAIN_TEXT.to_string(),
            media_url: Some("/not-image.jpg".to_string()),
            created_at,
            ..message("not-image")
        })
        .await
        .unwrap();
        dao.insert_message(&Message {
            category: PLAIN_IMAGE.to_string(),
            media_url: None,
            created_at,
            ..message("not-downloaded")
        })
        .await
        .unwrap();
        dao.insert_message(&Message {
            conversation_id: "other".to_string(),
            category: PLAIN_IMAGE.to_string(),
            media_url: Some("/other.jpg".to_string()),
            created_at,
            ..message("other")
        })
        .await
        .unwrap();

        let items = dao
            .list_image_items_around("conversation", "beta", 1, 1)
            .await
            .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item.message_id.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "beta", "gamma"]
        );
        assert_eq!(items[0].media_url, "/alpha.jpg");
        assert_eq!(items[2].media_name.as_deref(), Some("gamma.jpg"));
        assert!(items.iter().all(|item| item.can_forward));

        dao.insert_message(&Message {
            category: PLAIN_IMAGE.to_string(),
            content: Some("invalid attachment metadata".to_string()),
            media_url: Some("/invalid.jpg".to_string()),
            status: MessageStatus::Sent,
            media_status: MediaStatus::Done,
            created_at,
            ..message("invalid")
        })
        .await
        .unwrap();
        let invalid = dao
            .list_image_items_around("conversation", "invalid", 0, 0)
            .await
            .unwrap();
        assert_eq!(invalid.len(), 1);
        assert!(!invalid[0].can_forward);
    }

    #[tokio::test]
    async fn lists_system_message_participant_name() {
        let (_directory, database) = test_database().await;
        sqlx::query("INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)")
            .bind("participant")
            .bind("1000")
            .bind("Alice")
            .execute(&database.message_dao.0)
            .await
            .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                category: "SYSTEM_CONVERSATION".to_string(),
                action: Some("EXPIRE".to_string()),
                content: Some("3600".to_string()),
                participant_id: Some("participant".to_string()),
                ..message("system")
            })
            .await
            .unwrap();

        let items = database
            .message_dao
            .list_items("conversation", None, None, 1)
            .await
            .unwrap();

        assert_eq!(items[0].participant_full_name.as_deref(), Some("Alice"));
        assert_eq!(items[0].expire_in, Some(3600));
    }

    #[tokio::test]
    async fn lists_only_incoming_unread_messages() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        for (message_id, user_id, status) in [
            ("sent", "other", MessageStatus::Sent),
            ("delivered", "other", MessageStatus::Delivered),
            ("read", "other", MessageStatus::Read),
            ("own", "me", MessageStatus::Sent),
        ] {
            dao.insert_message(&Message {
                message_id: message_id.to_string(),
                user_id: user_id.to_string(),
                status,
                ..message(message_id)
            })
            .await
            .unwrap();
        }

        let unread = dao.unread_message_ids("conversation", "me").await.unwrap();
        assert_eq!(unread, ["sent", "delivered"]);
    }

    #[tokio::test]
    async fn marks_read_and_persists_both_receipt_jobs_in_one_transaction() {
        let (_directory, database) = test_database().await;
        let now = Utc::now();
        database
            .conversation_dao
            .insert(&Conversation {
                conversation_id: "conversation".into(),
                owner_id: Some("other".into()),
                category: Some(ConversationCategory::Contact),
                name: String::new(),
                icon_url: String::new(),
                announcement: String::new(),
                code_url: String::new(),
                created_at: now,
                status: ConversationStatus::SUCCESS,
                mute_until: now,
                expire_in: 60,
            })
            .await
            .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "incoming".into(),
                user_id: "other".into(),
                status: MessageStatus::Sent,
                ..message("incoming")
            })
            .await
            .unwrap();
        database
            .expired_message_dao
            .insert("incoming", 60, None)
            .await
            .unwrap();

        assert!(database
            .message_dao
            .mark_conversation_read("conversation", "me")
            .await
            .unwrap());

        let status: MessageStatus =
            sqlx::query_scalar("SELECT status FROM messages WHERE message_id = 'incoming'")
                .fetch_one(&database.message_dao.0)
                .await
                .unwrap();
        assert_eq!(status, MessageStatus::Read);
        let expire_at: i64 = sqlx::query_scalar(
            "SELECT expire_at FROM expired_messages WHERE message_id = 'incoming'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert!(expire_at >= Utc::now().timestamp() + 59);
        let actions: Vec<String> = sqlx::query_scalar("SELECT action FROM jobs ORDER BY action")
            .fetch_all(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(
            actions,
            [
                ACKNOWLEDGE_MESSAGE_RECEIPTS.to_string(),
                CREATE_MESSAGE.to_string(),
            ]
        );
        assert!(!database
            .message_dao
            .mark_conversation_read("conversation", "me")
            .await
            .unwrap());
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
