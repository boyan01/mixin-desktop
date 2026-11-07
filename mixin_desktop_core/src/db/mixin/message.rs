use std::collections::{HashMap, HashSet};

use crate::db::{Executor, QueryBuilder};
use anyhow::Context;
use base64ct::{Base64, Encoding};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use sdk::blaze_message::{MessageStatus, CREATE_MESSAGE};
use sdk::message_category::{MESSAGE_PIN, MESSAGE_RECALL};
use sdk::{ACKNOWLEDGE_MESSAGE_RECEIPTS, RECALL_MESSAGE as RECALL_MESSAGE_ACTION};

use crate::db::datetime::DatabaseDateTime;
use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::job::Job;
use crate::db::mixin::transcript_message::{TranscriptMessage, TranscriptMessageDao};
use crate::db::mixin::util::{expand_var, BindList};
use crate::db::Error;

#[cfg(test)]
mod pinned_media_tests;
mod pinning;

#[cfg(test)]
mod quote_projection_tests;

#[cfg(test)]
mod notification_tests;

#[derive(Clone)]
pub struct MessageDao(pub(crate) crate::db::Pool);

#[derive(Debug, Clone, Default)]
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

fn encode_attachment_material(value: &Option<Vec<u8>>) -> Option<String> {
    value.as_deref().map(Base64::encode_string)
}

fn decode_attachment_material(value: Option<Vec<u8>>) -> Option<Vec<u8>> {
    let value = value?;
    let Ok(encoded) = std::str::from_utf8(&value) else {
        return Some(value);
    };
    Base64::decode_vec(encoded).ok().or(Some(value))
}

fn decode_message_attachment_material(mut message: Message) -> Message {
    message.media_key = decode_attachment_material(message.media_key);
    message.media_digest = decode_attachment_material(message.media_digest);
    message
}

#[derive(Debug, Clone)]
pub struct MessageListItem {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub sender_name: String,
    pub sender_identity_number: String,
    pub sender_avatar_url: String,
    pub sender_is_verified: bool,
    pub sender_membership: Option<String>,
    pub sender_relationship: String,
    pub sender_app_id: Option<String>,
    pub sender_is_scam: bool,
    pub sender_is_bot: bool,
    pub sender_participant_id: Option<String>,
    pub sender_role: Option<String>,
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

#[derive(Debug, Clone)]
pub struct MessageOrderInfo {
    pub message_id: String,
    pub row_id: i64,
    pub created_at: NaiveDateTime,
}

const MESSAGE_LIST_QUERY_PREFIX: &str = r#"
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       sender.membership AS sender_membership,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       sender_participant.user_id AS sender_participant_id,
       sender_participant.role AS sender_role,
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
       CASE WHEN quote_sticker.sticker_id IS NOT NULL
            THEN json_set(message.quote_content,
                '$.asset_url', quote_sticker.asset_url,
                '$.asset_type', quote_sticker.asset_type,
                '$.asset_width', quote_sticker.asset_width,
                '$.asset_height', quote_sticker.asset_height,
                '$.asset_name', quote_sticker.name)
            ELSE message.quote_content END AS quote_content,
       quote_user.membership AS quote_user_membership,
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
       shared_user.membership AS shared_user_membership,
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
LEFT JOIN participants sender_participant
       ON sender_participant.conversation_id = message.conversation_id
      AND sender_participant.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN users quote_user ON quote_user.user_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.user_id') END
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN stickers quote_sticker ON quote_sticker.sticker_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.sticker_id') END
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
"#;

#[derive(Debug, Clone)]
pub struct ImageMessageItem {
    pub message_id: String,
    pub created_at: NaiveDateTime,
    pub media_url: String,
    pub media_mime_type: Option<String>,
    pub media_name: Option<String>,
    pub thumb_image: Option<String>,
    pub can_forward: bool,
    pub user_id: String,
    pub user_full_name: String,
    pub user_identity_number: String,
    pub avatar_url: String,
}

impl MessageListItem {
    pub fn created_at_micros(&self) -> i64 {
        self.created_at.and_utc().timestamp_micros()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaStatus {
    Pending,
    Done,
    #[default]
    Canceled,
    Expired,
    Read,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct QuoteMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub user_full_name: Option<String>,
    pub user_identity_number: String,
    pub app_id: Option<String>,
    #[serde(rename = "type")]
    pub category: String,
    pub content: Option<String>,
    #[serde(rename = "createdAt")]
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
    message.user_id AS userId,
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
         LEFT JOIN users sender ON message.user_id = sender.user_id
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

#[derive(Debug, Clone)]
pub struct MiniMessageItem {
    pub message_id: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone)]
pub struct NotificationMessageItem {
    pub row_id: i64,
    pub message_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub sender_name: String,
    pub action: Option<String>,
    pub participant_id: Option<String>,
    pub participant_name: Option<String>,
    pub category: String,
    pub status: MessageStatus,
    pub content: Option<String>,
    pub quote_content: Option<String>,
    pub created_at: NaiveDateTime,
    pub conversation_name: String,
    pub conversation_category: String,
    pub is_muted: bool,
}

impl MessageDao {
    pub async fn notification_items_after(
        &self,
        current_user_id: &str,
        after_row_id: i64,
        limit: i64,
    ) -> Result<Vec<NotificationMessageItem>, Error> {
        Ok(crate::db::query_as::<NotificationMessageItem>(
            r#"
SELECT message.rowid AS row_id,
       message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       message.action,
       message.participant_id,
       participant.full_name AS participant_name,
       message.category,
       message.content,
       message.quote_content,
       message.status,
       message.created_at,
       CASE WHEN conversation.category = 'GROUP'
            THEN conversation.name ELSE COALESCE(owner.full_name, '') END AS conversation_name,
       COALESCE(conversation.category, '') AS conversation_category,
       CASE WHEN conversation.category = 'GROUP'
            THEN conversation.mute_until >= ?3
            ELSE COALESCE(owner.mute_until >= ?3, FALSE) END AS is_muted
FROM messages message
INNER JOIN conversations conversation
        ON conversation.conversation_id = message.conversation_id
LEFT JOIN users owner ON owner.user_id = conversation.owner_id
LEFT JOIN users sender ON sender.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
WHERE message.user_id != ?1
  AND message.rowid > ?2
ORDER BY message.rowid ASC
LIMIT ?4
            "#,
        )
        .bind(current_user_id)
        .bind(after_row_id)
        .bind(Utc::now().timestamp_millis())
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn latest_row_id(&self) -> Result<i64, Error> {
        Ok(
            crate::db::query_scalar("SELECT COALESCE(MAX(rowid), 0) FROM messages")
                .fetch_one(&self.0)
                .await?,
        )
    }

    pub async fn unread_message_ids(
        &self,
        conversation_id: &str,
        current_user_id: &str,
    ) -> Result<Vec<String>, Error> {
        let result = crate::db::query_scalar::<String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND user_id != ? \
             AND status IN ('SENT', 'DELIVERED')",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn first_unread_message_id(
        &self,
        conversation_id: &str,
        current_user_id: &str,
    ) -> Result<Option<String>, Error> {
        Ok(crate::db::query_scalar::<String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND user_id != ? \
             AND status IN ('SENT', 'DELIVERED') ORDER BY created_at ASC, rowid ASC LIMIT 1",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn mark_conversation_read(
        &self,
        conversation_id: &str,
        current_user_id: &str,
    ) -> Result<(bool, bool), Error> {
        let has_unread = crate::db::query_scalar::<bool>(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE conversation_id = ? AND user_id != ? \
             AND status IN ('SENT', 'DELIVERED'))",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_one(&self.0)
        .await?;

        if !has_unread {
            let last_read_message_id = crate::db::query_scalar::<String>(
                "SELECT message_id FROM messages WHERE conversation_id = ? AND status = 'READ' \
                 ORDER BY created_at DESC, rowid DESC LIMIT 1",
            )
            .bind(conversation_id)
            .fetch_optional(&self.0)
            .await?;
            let unseen: i64 = crate::db::query_scalar(
                "SELECT COUNT(1) FROM messages WHERE conversation_id = ? \
                 AND status IN ('SENT', 'DELIVERED') AND user_id != ?",
            )
            .bind(conversation_id)
            .bind(current_user_id)
            .fetch_one(&self.0)
            .await?;
            let already_current = crate::db::query_scalar::<bool>(
                "SELECT EXISTS(SELECT 1 FROM conversations WHERE conversation_id = ? \
                 AND last_read_message_id IS ? AND unseen_message_count IS ?)",
            )
            .bind(conversation_id)
            .bind(&last_read_message_id)
            .bind(unseen)
            .fetch_one(&self.0)
            .await?;
            if already_current {
                return Ok((false, false));
            }
            let conversation_changed = crate::db::query(
                "UPDATE conversations SET last_read_message_id = ?, unseen_message_count = ? \
                 WHERE conversation_id = ? AND \
                 (last_read_message_id IS NOT ? OR unseen_message_count IS NOT ?)",
            )
            .bind(&last_read_message_id)
            .bind(unseen)
            .bind(conversation_id)
            .bind(&last_read_message_id)
            .bind(unseen)
            .execute(&self.0)
            .await?
            .rows_affected()
                > 0;
            return Ok((false, conversation_changed));
        }

        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let message_ids = crate::db::query_scalar::<String>(
            "UPDATE messages SET status = 'READ' WHERE conversation_id = ? AND user_id != ? \
             AND status IN ('SENT', 'DELIVERED') RETURNING message_id",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_all(&mut *transaction)
        .await?;
        let messages_changed = !message_ids.is_empty();

        let now = Utc::now().timestamp();
        for chunk in message_ids.chunks(MARK_LIMIT - 1) {
            let sql = format!(
                "UPDATE expired_messages SET expire_at = CAST((?1 + expire_in) AS INTEGER) \
                 WHERE (expire_at > (?1 + expire_in) OR expire_at IS NULL) \
                 AND message_id IN ({})",
                crate::db::mixin::util::expand_var_with_index(2, chunk.len())
            );
            crate::db::query(sqlx::AssertSqlSafe(sql))
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
                crate::db::query_as::<(String, Option<i64>)>(sqlx::AssertSqlSafe(sql))
                    .bind_list(chunk)
                    .fetch_all(&mut *transaction)
                    .await?,
            );
        }

        let last_read_message_id = crate::db::query_scalar::<String>(
            "SELECT message_id FROM messages WHERE conversation_id = ? AND status = 'READ' \
             ORDER BY created_at DESC, rowid DESC LIMIT 1",
        )
        .bind(conversation_id)
        .fetch_optional(&mut *transaction)
        .await?;
        let unseen: i64 = crate::db::query_scalar(
            "SELECT COUNT(1) FROM messages WHERE conversation_id = ? \
             AND status IN ('SENT', 'DELIVERED') AND user_id != ?",
        )
        .bind(conversation_id)
        .bind(current_user_id)
        .fetch_one(&mut *transaction)
        .await?;
        let conversation_changed = crate::db::query(
            "UPDATE conversations SET last_read_message_id = ?, unseen_message_count = ? \
             WHERE conversation_id = ? AND \
             (last_read_message_id IS NOT ? OR unseen_message_count IS NOT ?)",
        )
        .bind(&last_read_message_id)
        .bind(unseen)
        .bind(conversation_id)
        .bind(&last_read_message_id)
        .bind(unseen)
        .execute(&mut *transaction)
        .await?
        .rows_affected()
            > 0;

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
            let mut query_builder: QueryBuilder = QueryBuilder::new(
                "INSERT OR REPLACE INTO jobs \
                 (job_id, action, created_at, order_id, priority, user_id, conversation_id, \
                 resend_message_id, run_count, blaze_message) ",
            );
            query_builder.push_values(jobs, |mut builder, job| {
                builder
                    .push_bind(&job.job_id)
                    .push_bind(&job.action)
                    .push_bind(job.created_at.and_utc().timestamp_millis())
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
        Ok((messages_changed, conversation_changed))
    }

    pub async fn list_items(
        &self,
        conversation_id: &str,
        before_created_at_millis: Option<i64>,
        before_message_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        self.list_items_in_range(
            conversation_id,
            before_created_at_millis,
            before_message_id,
            limit.clamp(1, 200),
            (None, None),
        )
        .await
    }

    pub(crate) async fn list_items_in_range(
        &self,
        conversation_id: &str,
        before_created_at_millis: Option<i64>,
        before_message_id: Option<&str>,
        limit: i64,
        range_millis: (Option<i64>, Option<i64>),
    ) -> Result<Vec<MessageListItem>, Error> {
        let mut query = QueryBuilder::new(
            r#"
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       sender.membership AS sender_membership,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       sender_participant.user_id AS sender_participant_id,
       sender_participant.role AS sender_role,
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
       CASE WHEN quote_sticker.sticker_id IS NOT NULL
            THEN json_set(message.quote_content,
                '$.asset_url', quote_sticker.asset_url,
                '$.asset_type', quote_sticker.asset_type,
                '$.asset_width', quote_sticker.asset_width,
                '$.asset_height', quote_sticker.asset_height,
                '$.asset_name', quote_sticker.name)
            ELSE message.quote_content END AS quote_content,
       quote_user.membership AS quote_user_membership,
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
       shared_user.membership AS shared_user_membership,
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
LEFT JOIN participants sender_participant
       ON sender_participant.conversation_id = message.conversation_id
      AND sender_participant.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN users quote_user ON quote_user.user_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.user_id') END
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN stickers quote_sticker ON quote_sticker.sticker_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.sticker_id') END
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
WHERE message.conversation_id =
            "#,
        );
        query.push_bind(conversation_id);
        if let Some(start) = range_millis.0 {
            query.push(" AND message.created_at >= ").push_bind(start);
        }
        if let Some(end) = range_millis.1 {
            query.push(" AND message.created_at < ").push_bind(end);
        }
        if let Some(before) = before_created_at_millis {
            query
                .push(" AND message.created_at <= ")
                .push_bind(before)
                .push(" AND (message.created_at < ")
                .push_bind(before)
                .push(" OR message.rowid < (SELECT rowid FROM messages WHERE message_id = ")
                .push_bind(before_message_id.unwrap_or_default())
                .push("))");
        }
        query
            .push(" ORDER BY message.created_at DESC, message.rowid DESC LIMIT ")
            .push_bind(limit.clamp(1, 201));
        Ok(query
            .build_query_as::<MessageListItem>()
            .fetch_all(&self.0)
            .await?)
    }

    pub async fn list_items_by_ids(
        &self,
        message_ids: &[String],
    ) -> Result<Vec<MessageListItem>, Error> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut items = Vec::with_capacity(message_ids.len());
        for chunk in message_ids.chunks(MARK_LIMIT) {
            let sql = format!(
                "{MESSAGE_LIST_QUERY_PREFIX} WHERE message.message_id IN ({})",
                expand_var(chunk.len()),
            );
            items.extend(
                crate::db::query_as::<MessageListItem>(sqlx::AssertSqlSafe(sql))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?,
            );
        }
        Ok(items)
    }

    pub(crate) async fn list_items_by_sender(
        &self,
        conversation_id: &str,
        sender_id: &str,
        categories: &[String],
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MessageListItem>, Error> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let anchor = if let Some(cursor) = cursor {
            let anchor = crate::db::query_as::<(i64, i64)>(
                "SELECT created_at, rowid FROM messages WHERE conversation_id = ? AND user_id = ? AND message_id = ?",
            ).bind(conversation_id).bind(sender_id).bind(cursor).fetch_optional(&self.0).await?;
            let Some(anchor) = anchor else {
                return Ok(Vec::new());
            };
            Some(anchor)
        } else {
            None
        };
        let mut query = QueryBuilder::new(MESSAGE_LIST_QUERY_PREFIX);
        query
            .push(" WHERE message.conversation_id = ")
            .push_bind(conversation_id)
            .push(" AND message.user_id = ")
            .push_bind(sender_id);
        if !categories.is_empty() {
            query.push(" AND message.category IN (");
            let mut values = query.separated(", ");
            for category in categories {
                values.push_bind(category);
            }
            values.push_unseparated(")");
        }
        if let Some((created_at, rowid)) = anchor {
            query
                .push(" AND (message.created_at < ")
                .push_bind(created_at)
                .push(" OR (message.created_at = ")
                .push_bind(created_at)
                .push(" AND message.rowid < ")
                .push_bind(rowid)
                .push("))");
        }
        query
            .push(" ORDER BY message.created_at DESC, message.rowid DESC LIMIT ")
            .push_bind(limit.min(200));
        Ok(query
            .build_query_as::<MessageListItem>()
            .fetch_all(&self.0)
            .await?)
    }

    pub async fn message_order_info(
        &self,
        message_id: &str,
    ) -> Result<Option<MessageOrderInfo>, Error> {
        Ok(crate::db::query_as::<MessageOrderInfo>(
            "SELECT message_id, rowid AS row_id, created_at \
             FROM messages WHERE message_id = ? LIMIT 1",
        )
        .bind(message_id)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn message_ids_before(
        &self,
        conversation_id: &str,
        anchor: &MessageOrderInfo,
        limit: i64,
    ) -> Result<Vec<String>, Error> {
        Ok(crate::db::query_scalar(
            r#"
SELECT message_id
FROM messages
WHERE conversation_id = ?
  AND (created_at < ? OR (created_at = ? AND rowid < ?))
ORDER BY created_at DESC, rowid DESC
LIMIT ?
            "#,
        )
        .bind(conversation_id)
        .bind(anchor.created_at.and_utc().timestamp_millis())
        .bind(anchor.created_at.and_utc().timestamp_millis())
        .bind(anchor.row_id)
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn message_ids_after(
        &self,
        conversation_id: &str,
        anchor: &MessageOrderInfo,
        limit: i64,
    ) -> Result<Vec<String>, Error> {
        Ok(crate::db::query_scalar(
            r#"
SELECT message_id
FROM messages
WHERE conversation_id = ?
  AND (created_at > ? OR (created_at = ? AND rowid > ?))
ORDER BY created_at ASC, rowid ASC
LIMIT ?
            "#,
        )
        .bind(conversation_id)
        .bind(anchor.created_at.and_utc().timestamp_millis())
        .bind(anchor.created_at.and_utc().timestamp_millis())
        .bind(anchor.row_id)
        .bind(limit.clamp(1, 200))
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn list_pinned_items(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageListItem>, Error> {
        self.pinned_items(conversation_id, 0, i64::MAX).await
    }

    pub async fn list_pinned_items_page(
        &self,
        conversation_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        self.pinned_items(conversation_id, offset.max(0), limit.clamp(1, 200))
            .await
    }

    pub async fn pinned_message_count(&self, conversation_id: &str) -> Result<i64, Error> {
        Ok(crate::db::query_scalar("SELECT COUNT(*) FROM pin_messages pin INNER JOIN messages message ON message.message_id = pin.message_id WHERE pin.conversation_id = ?")
            .bind(conversation_id).fetch_one(&self.0).await?)
    }

    async fn pinned_items(
        &self,
        conversation_id: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        Ok(crate::db::query_as::< MessageListItem>(
            r#"
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       sender.membership AS sender_membership,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       sender_participant.user_id AS sender_participant_id,
       sender_participant.role AS sender_role,
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
       CASE WHEN quote_sticker.sticker_id IS NOT NULL
            THEN json_set(message.quote_content,
                '$.asset_url', quote_sticker.asset_url,
                '$.asset_type', quote_sticker.asset_type,
                '$.asset_width', quote_sticker.asset_width,
                '$.asset_height', quote_sticker.asset_height,
                '$.asset_name', quote_sticker.name)
            ELSE message.quote_content END AS quote_content,
       quote_user.membership AS quote_user_membership,
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
       shared_user.membership AS shared_user_membership,
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
LEFT JOIN participants sender_participant
       ON sender_participant.conversation_id = message.conversation_id
      AND sender_participant.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN users quote_user ON quote_user.user_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.user_id') END
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN stickers quote_sticker ON quote_sticker.sticker_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.sticker_id') END
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
ORDER BY message.created_at DESC, message.message_id DESC
LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(conversation_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.0)
        .await?)
    }

    /// A stable adjacent page in timeline order (newest first), excluding its anchor.
    pub async fn list_items_relative(
        &self,
        conversation_id: &str,
        anchor_message_id: &str,
        older: bool,
        limit: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        let limit = limit.min(100);
        let mut rows = self
            .list_items_around(
                conversation_id,
                anchor_message_id,
                if older { limit } else { 0 },
                if older { 0 } else { limit },
            )
            .await?;
        rows.retain(|row| row.message_id != anchor_message_id);
        rows.reverse();
        Ok(rows)
    }

    pub async fn list_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        let Some(anchor) = crate::db::query_as::<MessageOrderInfo>(
            "SELECT message_id, rowid AS row_id, created_at FROM messages \
             WHERE conversation_id = ? AND message_id = ? LIMIT 1",
        )
        .bind(conversation_id)
        .bind(target_message_id)
        .fetch_optional(&self.0)
        .await?
        else {
            return Ok(Vec::new());
        };
        let result = crate::db::query_as::< MessageListItem>(
            r#"
WITH message_window AS (
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate
        WHERE candidate.conversation_id = ?1
          AND (
              candidate.created_at < ?3
              OR (candidate.created_at = ?3 AND candidate.rowid < ?4)
          )
        ORDER BY candidate.created_at DESC, candidate.rowid DESC
        LIMIT ?5
    )
    UNION ALL
    SELECT ?2 AS message_id
    UNION ALL
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate
        WHERE candidate.conversation_id = ?1
          AND (
              candidate.created_at > ?3
              OR (candidate.created_at = ?3 AND candidate.rowid > ?4)
          )
        ORDER BY candidate.created_at ASC, candidate.rowid ASC
        LIMIT ?6
    )
)
SELECT message.message_id,
       message.conversation_id,
       message.user_id,
       COALESCE(sender.full_name, '') AS sender_name,
       COALESCE(sender.identity_number, '') AS sender_identity_number,
       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
       sender.membership AS sender_membership,
       COALESCE(sender.relationship, '') AS sender_relationship,
       sender.app_id AS sender_app_id,
       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
       CASE WHEN COALESCE(sender.app_id, '') != '' THEN TRUE ELSE FALSE END AS sender_is_bot,
       sender_participant.user_id AS sender_participant_id,
       sender_participant.role AS sender_role,
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
       CASE WHEN quote_sticker.sticker_id IS NOT NULL
            THEN json_set(message.quote_content,
                '$.asset_url', quote_sticker.asset_url,
                '$.asset_type', quote_sticker.asset_type,
                '$.asset_width', quote_sticker.asset_width,
                '$.asset_height', quote_sticker.asset_height,
                '$.asset_name', quote_sticker.name)
            ELSE message.quote_content END AS quote_content,
       quote_user.membership AS quote_user_membership,
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
       shared_user.membership AS shared_user_membership,
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
LEFT JOIN participants sender_participant
       ON sender_participant.conversation_id = message.conversation_id
      AND sender_participant.user_id = message.user_id
LEFT JOIN users participant ON participant.user_id = message.participant_id
LEFT JOIN conversations conversation ON conversation.conversation_id = message.conversation_id
LEFT JOIN users shared_user ON shared_user.user_id = message.shared_user_id
LEFT JOIN users quote_user ON quote_user.user_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.user_id') END
LEFT JOIN stickers sticker ON sticker.sticker_id = message.sticker_id
LEFT JOIN stickers quote_sticker ON quote_sticker.sticker_id = CASE WHEN json_valid(message.quote_content) THEN json_extract(message.quote_content, '$.sticker_id') END
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
ORDER BY message.created_at ASC, message.rowid ASC
            "#,
        )
        .bind(conversation_id)
        .bind(target_message_id)
        .bind(anchor.created_at.and_utc().timestamp_millis())
        .bind(anchor.row_id)
        .bind(before.clamp(0, 100))
        .bind(after.clamp(0, 100))
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    /// Select eligible audio before applying the bounded neighbor window.
    pub async fn list_audio_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        self.audio_items_around(conversation_id, target_message_id, before, after, false)
            .await
    }

    pub async fn list_pinned_audio_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<MessageListItem>, Error> {
        self.audio_items_around(conversation_id, target_message_id, before, after, true)
            .await
    }

    async fn audio_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
        pinned_only: bool,
    ) -> Result<Vec<MessageListItem>, Error> {
        let ids = crate::db::query_scalar::<String>(
            r#"
WITH eligible AS (
    SELECT message_id, created_at, rowid AS row_id FROM messages
    WHERE conversation_id = ?1
      AND category IN ('PLAIN_AUDIO', 'SIGNAL_AUDIO', 'ENCRYPTED_AUDIO')
      AND media_status IN ('DONE', 'READ') AND COALESCE(media_url, '') != ''
      AND (?5 = FALSE OR EXISTS (
          SELECT 1 FROM pin_messages pin
          WHERE pin.message_id = messages.message_id AND pin.conversation_id = ?1
      ))
), anchor AS (
    SELECT * FROM eligible WHERE message_id = ?2
), audio_window AS (
    SELECT * FROM (
        SELECT candidate.* FROM eligible candidate, anchor
        WHERE candidate.created_at < anchor.created_at
           OR (candidate.created_at = anchor.created_at AND candidate.row_id < anchor.row_id)
        ORDER BY candidate.created_at DESC, candidate.row_id DESC LIMIT ?3
    )
    UNION ALL SELECT * FROM anchor
    UNION ALL SELECT * FROM (
        SELECT candidate.* FROM eligible candidate, anchor
        WHERE candidate.created_at > anchor.created_at
           OR (candidate.created_at = anchor.created_at AND candidate.row_id > anchor.row_id)
        ORDER BY candidate.created_at ASC, candidate.row_id ASC LIMIT ?4
    )
)
SELECT message_id FROM audio_window ORDER BY created_at ASC, row_id ASC
            "#,
        )
        .bind(conversation_id)
        .bind(target_message_id)
        .bind(before.clamp(0, 100))
        .bind(after.clamp(0, 100))
        .bind(pinned_only)
        .fetch_all(&self.0)
        .await?;
        let mut items = self
            .list_items_by_ids(&ids)
            .await?
            .into_iter()
            .map(|item| (item.message_id.clone(), item))
            .collect::<HashMap<_, _>>();
        Ok(ids.into_iter().filter_map(|id| items.remove(&id)).collect())
    }

    pub async fn list_image_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<ImageMessageItem>, Error> {
        self.image_items_around(conversation_id, target_message_id, before, after, false)
            .await
    }

    pub async fn list_pinned_image_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
    ) -> Result<Vec<ImageMessageItem>, Error> {
        self.image_items_around(conversation_id, target_message_id, before, after, true)
            .await
    }

    async fn image_items_around(
        &self,
        conversation_id: &str,
        target_message_id: &str,
        before: i64,
        after: i64,
        pinned_only: bool,
    ) -> Result<Vec<ImageMessageItem>, Error> {
        Ok(crate::db::query_as::<ImageMessageItem>(
            r#"
WITH anchor AS (
    SELECT message_id, rowid AS row_id, created_at FROM messages
    WHERE conversation_id = ?1 AND message_id = ?2
      AND category IN ('SIGNAL_IMAGE', 'PLAIN_IMAGE', 'ENCRYPTED_IMAGE')
      AND COALESCE(media_url, '') != ''
      AND (?5 = FALSE OR EXISTS (SELECT 1 FROM pin_messages pin
           WHERE pin.message_id = messages.message_id AND pin.conversation_id = ?1))
), image_window AS (
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate, anchor
        WHERE candidate.conversation_id = ?1
          AND candidate.category IN ('SIGNAL_IMAGE', 'PLAIN_IMAGE', 'ENCRYPTED_IMAGE')
          AND COALESCE(candidate.media_url, '') != ''
          AND (?5 = FALSE OR EXISTS (SELECT 1 FROM pin_messages pin
               WHERE pin.message_id = candidate.message_id AND pin.conversation_id = ?1))
          AND (
              candidate.created_at < anchor.created_at
              OR (candidate.created_at = anchor.created_at AND candidate.rowid < anchor.row_id)
          )
        ORDER BY candidate.created_at DESC, candidate.rowid DESC
        LIMIT ?3
    )
    UNION ALL
    SELECT message_id FROM anchor
    UNION ALL
    SELECT message_id FROM (
        SELECT candidate.message_id
        FROM messages candidate, anchor
        WHERE candidate.conversation_id = ?1
          AND candidate.category IN ('SIGNAL_IMAGE', 'PLAIN_IMAGE', 'ENCRYPTED_IMAGE')
          AND COALESCE(candidate.media_url, '') != ''
          AND (?5 = FALSE OR EXISTS (SELECT 1 FROM pin_messages pin
               WHERE pin.message_id = candidate.message_id AND pin.conversation_id = ?1))
          AND (
              candidate.created_at > anchor.created_at
              OR (candidate.created_at = anchor.created_at AND candidate.rowid > anchor.row_id)
          )
        ORDER BY candidate.created_at ASC, candidate.rowid ASC
        LIMIT ?4
    )
)
SELECT message.message_id,
       message.created_at,
       message.media_url,
       message.media_mime_type,
       message.name AS media_name,
       message.thumb_image,
       CASE
           WHEN message.status IN ('SENT', 'DELIVERED', 'READ')
            AND message.media_status IN ('DONE', 'READ')
            AND json_valid(message.content)
            AND COALESCE(json_extract(message.content, '$.shareable'), TRUE) != FALSE
           THEN TRUE ELSE FALSE
       END AS can_forward
       ,message.user_id
       ,COALESCE(user.full_name, '') AS user_full_name
       ,COALESCE(user.identity_number, '') AS user_identity_number
       ,COALESCE(user.avatar_url, '') AS avatar_url
FROM image_window
INNER JOIN messages message ON message.message_id = image_window.message_id
LEFT JOIN users user ON user.user_id = message.user_id
ORDER BY message.created_at ASC, message.rowid ASC
            "#,
        )
        .bind(conversation_id)
        .bind(target_message_id)
        .bind(before.clamp(0, 100))
        .bind(after.clamp(0, 100))
        .bind(pinned_only)
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
        let result = crate::db::query_as::<QuoteMessage>(sqlx::AssertSqlSafe(query_str))
            .bind(message_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn find_message_by_id(&self, message_id: &String) -> Result<Option<Message>, Error> {
        let result = crate::db::query_as::<Message>("SELECT * FROM messages WHERE message_id = ?")
            .bind(message_id)
            .fetch_optional(&self.0)
            .await?;
        Ok(result.map(decode_message_attachment_material))
    }

    pub async fn conversation_id_by_message_id(
        &self,
        message_id: &str,
    ) -> Result<Option<String>, Error> {
        Ok(
            crate::db::query_scalar("SELECT conversation_id FROM messages WHERE message_id = ?")
                .bind(message_id)
                .fetch_optional(&self.0)
                .await?,
        )
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
                crate::db::query_as::<Message>(sqlx::AssertSqlSafe(query))
                    .bind_list(chunk)
                    .fetch_all(&self.0)
                    .await?
                    .into_iter()
                    .map(decode_message_attachment_material),
            );
        }
        Ok(messages)
    }

    pub async fn is_message_exits(&self, message_id: &String) -> Result<bool, Error> {
        let result = crate::db::query_scalar::<bool>(
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        Self::insert_outgoing_message_with(&mut transaction, message, job).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_pending_outgoing_message(&self, message: &Message) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        Self::ensure_can_send_with(&mut transaction, message).await?;
        insert_message_with(&mut *transaction, message).await?;
        crate::db::query(
            "UPDATE conversations SET last_message_id = ?, last_message_created_at = ?, \
             draft = '' WHERE conversation_id = ?",
        )
        .bind(&message.message_id)
        .bind(message.created_at.and_utc().timestamp_millis())
        .bind(&message.conversation_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_outgoing_message_with_transcripts(
        &self,
        message: &Message,
        job: &Job,
        transcripts: &[TranscriptMessage],
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        TranscriptMessageDao::insert_all_with(&mut transaction, transcripts).await?;
        Self::insert_outgoing_message_with(&mut transaction, message, job).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub(crate) async fn insert_outgoing_batch_with_transcripts(
        &self,
        messages: &[(&Message, &Job, Option<&[TranscriptMessage]>)],
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        for (message, job, transcripts) in messages {
            if let Some(transcripts) = transcripts {
                TranscriptMessageDao::insert_all_with(&mut transaction, transcripts).await?;
            }
            Self::insert_outgoing_message_with(&mut transaction, message, job).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    /// Account startup runs this before any attachment transfer can be active.
    pub(crate) async fn recover_interrupted_uploads(&self, account_id: &str) -> Result<(), Error> {
        crate::db::query(
            "UPDATE messages SET media_status = 'CANCELED' \
             WHERE user_id = ? AND status = 'SENDING' AND media_status = 'PENDING' \
             AND category IN ('PLAIN_IMAGE', 'SIGNAL_IMAGE', 'ENCRYPTED_IMAGE', \
              'PLAIN_VIDEO', 'SIGNAL_VIDEO', 'ENCRYPTED_VIDEO', \
              'PLAIN_AUDIO', 'SIGNAL_AUDIO', 'ENCRYPTED_AUDIO', \
              'PLAIN_DATA', 'SIGNAL_DATA', 'ENCRYPTED_DATA')",
        )
        .bind(account_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    async fn ensure_can_send_with(
        transaction: &mut crate::db::Transaction<'_>,
        message: &Message,
    ) -> Result<(), Error> {
        if !super::conversation::ConversationDao::can_send_with(
            &mut **transaction,
            &message.conversation_id,
            &message.user_id,
        )
        .await?
        {
            return Err(Error::PermissionDenied);
        }
        Ok(())
    }

    async fn insert_outgoing_message_with(
        transaction: &mut crate::db::Transaction<'_>,
        message: &Message,
        job: &Job,
    ) -> Result<(), Error> {
        Self::ensure_can_send_with(transaction, message).await?;
        insert_message_with(&mut **transaction, message).await?;
        crate::db::query(
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

    pub(super) async fn insert_job_with(
        transaction: &mut crate::db::Transaction<'_>,
        job: &Job,
    ) -> Result<(), Error> {
        let statement = if job.action == RECALL_MESSAGE_ACTION {
            r#"INSERT OR IGNORE INTO jobs (job_id, action, created_at, order_id, priority, user_id,
             conversation_id, resend_message_id, run_count, blaze_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        } else {
            r#"INSERT OR REPLACE INTO jobs (job_id, action, created_at, order_id, priority, user_id,
             conversation_id, resend_message_id, run_count, blaze_message)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        };
        crate::db::query(statement)
            .bind(&job.job_id)
            .bind(&job.action)
            .bind(job.created_at.and_utc().timestamp_millis())
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        if let Some(content) = content {
            crate::db::query("UPDATE messages SET content = ? WHERE message_id = ?")
                .bind(content)
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
        }
        let target = match status {
            MessageStatus::Sent => "SENT",
            MessageStatus::Failed => "FAILED",
            _ => {
                return Err(
                    anyhow::anyhow!("invalid sending completion status: {status:?}").into(),
                );
            }
        };
        crate::db::query(
            "UPDATE messages SET status = ? WHERE message_id = ? AND status = 'SENDING'",
        )
        .bind(target)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        if status == MessageStatus::Sent && expire_in > 0 {
            crate::db::query(
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
        crate::db::query("DELETE FROM jobs WHERE job_id = ?")
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
        let _ =
            crate::db::query("UPDATE messages SET content = ?, status = ? WHERE message_id = ?")
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
        crate::db::query(
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
        let _ = crate::db::query(
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
        .bind(encode_attachment_material(&update.media_digest))
        .bind(encode_attachment_material(&update.media_key))
        .bind(encode_attachment_material(&update.media_waveform))
        .bind(&update.caption)
        .bind(&update.name)
        .bind(&update.thumb_image)
        .bind(&update.media_duration)
        .bind(message_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn complete_pending_attachment(
        &self,
        message_id: &str,
        media_url: &str,
        update: &AttachmentMessageUpdate,
        job: &Job,
    ) -> Result<bool, Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let result = crate::db::query(
            r#"UPDATE messages SET
         status = ?, content = ?, media_url = ?, media_mime_type = ?, media_size = ?,
         media_status = ?, media_width = ?, media_height = ?, media_digest = ?, media_key = ?,
         media_waveform = ?, caption = ?, name = ?, thumb_image = ?, media_duration = ?
          WHERE message_id = ? AND media_status = ?"#,
        )
        .bind(update.status)
        .bind(&update.content)
        .bind(media_url)
        .bind(&update.media_mime_type)
        .bind(update.media_size)
        .bind(&update.media_status)
        .bind(update.media_width)
        .bind(update.media_height)
        .bind(encode_attachment_material(&update.media_digest))
        .bind(encode_attachment_material(&update.media_key))
        .bind(encode_attachment_material(&update.media_waveform))
        .bind(&update.caption)
        .bind(&update.name)
        .bind(&update.thumb_image)
        .bind(&update.media_duration)
        .bind(message_id)
        .bind(MediaStatus::Pending)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() == 0 {
            transaction.rollback().await?;
            return Ok(false);
        }
        Self::insert_job_with(&mut transaction, job).await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn complete_attachment_retry_if_pending(
        &self,
        message_id: &str,
        update: &AttachmentMessageUpdate,
    ) -> Result<bool, Error> {
        let result = crate::db::query(
            r#"UPDATE messages SET
         status = ?, content = ?, media_mime_type = ?, media_size = ?, media_status = ?,
         media_width = ?, media_height = ?, media_digest = ?, media_key = ?, media_waveform = ?,
         caption = ?, name = ?, thumb_image = ?, media_duration = ?
          WHERE message_id = ? AND media_status = ?"#,
        )
        .bind(update.status)
        .bind(&update.content)
        .bind(&update.media_mime_type)
        .bind(update.media_size)
        .bind(&update.media_status)
        .bind(update.media_width)
        .bind(update.media_height)
        .bind(encode_attachment_material(&update.media_digest))
        .bind(encode_attachment_material(&update.media_key))
        .bind(encode_attachment_material(&update.media_waveform))
        .bind(&update.caption)
        .bind(&update.name)
        .bind(&update.thumb_image)
        .bind(&update.media_duration)
        .bind(message_id)
        .bind(MediaStatus::Pending)
        .execute(&self.0)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn update_sticker_message(
        &self,
        message_id: &str,
        sticker_id: String,
        status: MessageStatus,
    ) -> Result<(), Error> {
        let _ =
            crate::db::query("UPDATE messages SET sticker_id = ?, status = ? WHERE message_id = ?")
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
        crate::db::query("UPDATE messages SET shared_user_id = ?, status = ? WHERE message_id = ?")
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
        let _ = crate::db::query(
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

    /// Failure recovery must not overwrite an already completed upload/job.
    pub async fn cancel_pending_transfer(&self, message_id: &str) -> Result<bool, Error> {
        Ok(crate::db::query(
            "UPDATE messages SET media_status = 'CANCELED' WHERE message_id = ? \
             AND media_status = 'PENDING'",
        )
        .bind(message_id)
        .execute(&self.0)
        .await?
        .rows_affected()
            > 0)
    }

    /// Failure recovery for an accepted outgoing send must preserve completed jobs.
    pub async fn cancel_pending_attachment(&self, message_id: &str) -> Result<bool, Error> {
        Ok(crate::db::query(
            "UPDATE messages SET media_status = 'CANCELED' WHERE message_id = ? \
             AND media_status = 'PENDING' AND status = 'SENDING'",
        )
        .bind(message_id)
        .execute(&self.0)
        .await?
        .rows_affected()
            > 0)
    }

    pub async fn update_media_status(
        &self,
        message_id: &str,
        status: MediaStatus,
    ) -> Result<(), Error> {
        crate::db::query("UPDATE messages SET media_status = ? WHERE message_id = ?")
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
        crate::db::query(
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
        let result = crate::db::query(
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
        let message_ids = crate::db::query_scalar::<String>(
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
                let mut query_builder: QueryBuilder =
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
                crate::db::query_as::<MiniMessageItem>(sqlx::AssertSqlSafe(query_str))
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
            let _ = crate::db::query(sqlx::AssertSqlSafe(query))
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
        let message = crate::db::query_as::<Message>(
            "SELECT * FROM messages WHERE message_id = ? AND status = ? \
             AND content IS NOT NULL LIMIT 1",
        )
        .bind(message_id)
        .bind(MessageStatus::Sending)
        .fetch_optional(&self.0)
        .await?;
        Ok(message.map(decode_message_attachment_material))
    }

    pub async fn update_message_category(
        &self,
        message_id: &str,
        category: &str,
    ) -> Result<u64, Error> {
        let result = crate::db::query("UPDATE messages SET category = ? WHERE message_id = ?")
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
        let result = crate::db::query("UPDATE messages SET status = ? WHERE message_id = ?")
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
        let result = crate::db::query(
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        for message_id in message_ids {
            let exists = crate::db::query_scalar::<bool>(
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
            crate::db::query(
                "UPDATE messages SET content = NULL WHERE conversation_id = ? \
                 AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
            )
            .bind(conversation_id)
            .bind(MESSAGE_PIN)
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
            crate::db::mixin::message_fts::delete_message_fts(&mut transaction, message_id).await?;
            for table in ["pin_messages", "expired_messages", "message_mentions"] {
                let query = format!("DELETE FROM {table} WHERE message_id = ?");
                crate::db::query(sqlx::AssertSqlSafe(query))
                    .bind(message_id)
                    .execute(&mut *transaction)
                    .await?;
            }
            crate::db::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
            deleted += crate::db::query(
                "DELETE FROM messages WHERE conversation_id = ? AND message_id = ?",
            )
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        crate::db::mixin::message_fts::delete_conversation_fts(&mut transaction, conversation_id)
            .await?;
        crate::db::query(
            "DELETE FROM transcript_messages WHERE transcript_id IN (\
             SELECT message_id FROM messages WHERE conversation_id = ?)",
        )
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
        crate::db::query(
            "DELETE FROM expired_messages WHERE message_id IN (\
             SELECT message_id FROM messages WHERE conversation_id = ?)",
        )
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
        for table in ["pin_messages", "message_mentions", "messages"] {
            let query = format!("DELETE FROM {table} WHERE conversation_id = ?");
            crate::db::query(sqlx::AssertSqlSafe(query))
                .bind(conversation_id)
                .execute(&mut *transaction)
                .await?;
        }
        Self::update_conversation_last_message_with(&mut transaction, conversation_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_recall_jobs(
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        for message_id in message_ids {
            let exists = crate::db::query_scalar::<bool>(
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
        for job in jobs {
            Self::insert_job_with(&mut transaction, job).await?;
        }
        transaction.commit().await?;
        Ok(jobs.len() as u64)
    }

    pub async fn set_message_pinned_with_job(
        &self,
        conversation_id: &str,
        message_id: &str,
        pinned: bool,
        created_at: DateTime<Utc>,
        pin_event: Option<&Message>,
        job: &Job,
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let exists = crate::db::query_scalar::<bool>(
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
        if pinned != pin_event.is_some() {
            return Err(anyhow::anyhow!("pin event does not match pin state").into());
        }
        Self::insert_job_with(&mut transaction, job).await?;
        if pinned {
            crate::db::query(
                "INSERT OR REPLACE INTO pin_messages (message_id, conversation_id, created_at) \
                 VALUES (?, ?, ?)",
            )
            .bind(message_id)
            .bind(conversation_id)
            .bind(created_at.timestamp_millis())
            .execute(&mut *transaction)
            .await?;
            let pin_event = pin_event.expect("pin event checked above");
            insert_message_with(&mut *transaction, pin_event).await?;
            Self::update_conversation_last_message_with(&mut transaction, conversation_id).await?;
        } else {
            crate::db::query("DELETE FROM pin_messages WHERE message_id = ?")
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn update_conversation_last_message_with(
        transaction: &mut crate::db::Transaction<'_>,
        conversation_id: &str,
    ) -> Result<(), Error> {
        let latest = crate::db::query_as::<(String, DatabaseDateTime)>(
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
                    Some(DateTime::<Utc>::from(created_at).timestamp_millis()),
                )
            })
            .unwrap_or_default();
        crate::db::query(
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;

        let result = crate::db::query(
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

        crate::db::query(
            "UPDATE messages SET content = NULL WHERE conversation_id = ? \
             AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
        )
        .bind(conversation_id)
        .bind(MESSAGE_PIN)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;

        crate::db::query("DELETE FROM pin_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        crate::db::mixin::message_fts::delete_message_fts(&mut transaction, message_id).await?;
        crate::db::query("DELETE FROM message_mentions WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        crate::db::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;

        crate::db::query(
            "UPDATE messages SET content = NULL WHERE conversation_id = ? \
             AND category = ? AND quote_message_id = ? AND content IS NOT NULL",
        )
        .bind(conversation_id)
        .bind(MESSAGE_PIN)
        .bind(message_id)
        .execute(&mut *transaction)
        .await?;
        crate::db::query("DELETE FROM pin_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        crate::db::query("DELETE FROM expired_messages WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        crate::db::query("DELETE FROM message_mentions WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;
        crate::db::mixin::message_fts::delete_message_fts(&mut transaction, message_id).await?;
        crate::db::query("DELETE FROM transcript_messages WHERE transcript_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;

        let result =
            crate::db::query("DELETE FROM messages WHERE conversation_id = ? AND message_id = ?")
                .bind(conversation_id)
                .bind(message_id)
                .execute(&mut *transaction)
                .await?;

        let latest = crate::db::query_as::<(String, DatabaseDateTime)>(
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
                    Some(DateTime::<Utc>::from(created_at).timestamp_millis()),
                )
            })
            .unwrap_or_default();

        crate::db::query(
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
        let result = crate::db::query_scalar::< String>(
            "SELECT message_id FROM messages INDEXED BY index_message_conversation_id_status_user_id \
             WHERE conversation_id = ? AND user_id = ? AND status = ? \
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

async fn insert_message_with<E>(executor: E, message: &Message) -> Result<(), sqlx::Error>
where
    E: Executor,
{
    crate::db::query(
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
    .bind(encode_attachment_material(&message.media_key))
    .bind(encode_attachment_material(&message.media_digest))
    .bind(&message.media_status)
    .bind(message.status)
    .bind(message.created_at.and_utc().timestamp_millis())
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
    use std::time::Duration;

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
        crate::db::query(
            "INSERT INTO conversations (conversation_id, created_at, status, category) \
             VALUES ('conversation', 0, 0, 'CONTACT')",
        )
        .execute(&database.message_dao.0)
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

    fn pin_event(message_id: &str, pinned_message_id: &str) -> Message {
        Message {
            message_id: message_id.to_string(),
            conversation_id: "conversation".to_string(),
            user_id: "sender".to_string(),
            category: MESSAGE_PIN.to_string(),
            content: Some(format!(
                r#"{{"category":"PLAIN_TEXT","message_id":"{pinned_message_id}","content":"hello"}}"#
            )),
            status: MessageStatus::Read,
            created_at: Utc::now().naive_utc(),
            quote_message_id: Some(pinned_message_id.to_string()),
            ..Message::default()
        }
    }

    #[tokio::test]
    async fn sender_search_without_keywords_filters_before_stable_cursor_paging() {
        let (_directory, database) = test_database().await;
        crate::db::query("INSERT INTO conversations (conversation_id, created_at, status) VALUES ('other-chat', 0, 0)").execute(&database.message_dao.0).await.unwrap();
        let dao = &database.message_dao;
        let created_at = Utc::now().naive_utc();
        for (id, sender, category, conversation) in [
            ("first", "sender", PLAIN_TEXT, "conversation"),
            ("picture", "sender", PLAIN_IMAGE, "conversation"),
            ("second", "sender", PLAIN_TEXT, "conversation"),
            ("other-user", "other", PLAIN_TEXT, "conversation"),
            ("other-chat", "sender", PLAIN_TEXT, "other-chat"),
            ("third", "sender", PLAIN_TEXT, "conversation"),
        ] {
            dao.insert_message(&Message {
                created_at,
                user_id: sender.into(),
                category: category.into(),
                conversation_id: conversation.into(),
                ..message(id)
            })
            .await
            .unwrap();
        }
        let categories = vec![PLAIN_TEXT.to_string()];
        let first = dao
            .list_items_by_sender("conversation", "sender", &categories, None, 2)
            .await
            .unwrap();
        assert_eq!(
            first
                .iter()
                .map(|row| row.message_id.as_str())
                .collect::<Vec<_>>(),
            ["third", "second"]
        );
        let second = dao
            .list_items_by_sender("conversation", "sender", &categories, Some("second"), 2)
            .await
            .unwrap();
        assert_eq!(
            second
                .iter()
                .map(|row| row.message_id.as_str())
                .collect::<Vec<_>>(),
            ["first"]
        );
        let all = dao
            .list_items_by_sender("conversation", "sender", &[], None, 10)
            .await
            .unwrap();
        assert_eq!(
            all.iter()
                .map(|row| row.message_id.as_str())
                .collect::<Vec<_>>(),
            ["third", "second", "picture", "first"]
        );
        for cursor in ["missing", "other-user", "other-chat"] {
            assert!(dao
                .list_items_by_sender("conversation", "sender", &[], Some(cursor), 2)
                .await
                .unwrap()
                .is_empty());
        }
        assert!(dao
            .list_items_by_sender("conversation", "sender", &[], None, 0)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn adjacent_timeline_pages_exclude_anchor_and_keep_stable_descending_order_for_ties() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        let created_at = Utc::now().naive_utc();
        for id in ["one", "two", "anchor", "four", "five"] {
            dao.insert_message(&Message {
                created_at,
                ..message(id)
            })
            .await
            .unwrap();
        }
        for (older, expected) in [(true, vec!["two", "one"]), (false, vec!["five", "four"])] {
            let rows = dao
                .list_items_relative("conversation", "anchor", older, 2)
                .await
                .unwrap();
            assert_eq!(
                rows.iter()
                    .map(|row| row.message_id.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        assert!(dao
            .list_items_relative("other", "anchor", true, 2)
            .await
            .unwrap()
            .is_empty());
        assert!(dao
            .list_items_relative("conversation", "missing", false, 2)
            .await
            .unwrap()
            .is_empty());
        assert!(dao
            .list_items_relative("conversation", "anchor", false, 0)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn attachment_completion_and_retry_store_binary_waveforms_as_base64_text() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        let waveform = vec![0, 127, 128, 255];
        let update = AttachmentMessageUpdate {
            status: MessageStatus::Sending,
            content: "attachment".into(),
            media_mime_type: "audio/ogg".into(),
            media_size: 42,
            media_status: MediaStatus::Done,
            media_width: None,
            media_height: None,
            media_digest: None,
            media_key: None,
            media_waveform: Some(waveform.clone()),
            caption: None,
            name: None,
            thumb_image: None,
            media_duration: Some("500".into()),
        };
        for id in ["ordinary", "pending", "retry"] {
            dao.insert_message(&Message {
                category: "SIGNAL_AUDIO".into(),
                media_status: MediaStatus::Pending,
                ..message(id)
            })
            .await
            .unwrap();
        }
        dao.update_attachment_message("ordinary", &update)
            .await
            .unwrap();
        let job = Job::create_sending_job("pending", "conversation", None, None, false, false, 0);
        assert!(dao
            .complete_pending_attachment("pending", "test.ogg", &update, &job)
            .await
            .unwrap());
        assert!(dao
            .complete_attachment_retry_if_pending("retry", &update)
            .await
            .unwrap());
        for id in ["ordinary", "pending", "retry"] {
            let stored: (String, String) = crate::db::query_as(
                "SELECT typeof(media_waveform), media_waveform FROM messages WHERE message_id = ?",
            )
            .bind(id)
            .fetch_one(&dao.0)
            .await
            .unwrap();
            assert_eq!(stored.0, "text");
            assert_eq!(Base64::decode_vec(&stored.1).unwrap(), waveform);
            let message = dao
                .find_message_by_id(&id.to_owned())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(message.media_waveform.as_deref(), Some(stored.1.as_str()));
        }
    }

    #[tokio::test]
    async fn failed_upload_is_retryable_without_overwriting_completed_attachment_state() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        for (id, media_status) in [
            ("pending", MediaStatus::Pending),
            ("completed", MediaStatus::Done),
        ] {
            dao.insert_message(&Message {
                category: "SIGNAL_DATA".into(),
                media_status,
                ..message(id)
            })
            .await
            .unwrap();
        }
        assert!(dao.cancel_pending_attachment("pending").await.unwrap());
        assert!(!dao.cancel_pending_attachment("completed").await.unwrap());
        assert!(!dao.cancel_pending_attachment("missing").await.unwrap());
        assert_eq!(
            dao.find_message_by_id(&"pending".to_owned())
                .await
                .unwrap()
                .unwrap()
                .media_status,
            MediaStatus::Canceled
        );
        assert_eq!(
            dao.find_message_by_id(&"completed".to_owned())
                .await
                .unwrap()
                .unwrap()
                .media_status,
            MediaStatus::Done
        );
    }

    #[tokio::test]
    async fn audio_neighbors_filter_before_limits_and_unread_uses_stable_message_order() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        let time = Utc::now().naive_utc();
        for (id, category, media_status, sender, status) in [
            (
                "read",
                "PLAIN_AUDIO",
                MediaStatus::Read,
                "other",
                MessageStatus::Read,
            ),
            (
                "self",
                "PLAIN_AUDIO",
                MediaStatus::Done,
                "me",
                MessageStatus::Sent,
            ),
            (
                "first",
                "SIGNAL_AUDIO",
                MediaStatus::Done,
                "other",
                MessageStatus::Sent,
            ),
            (
                "text",
                "PLAIN_TEXT",
                MediaStatus::Done,
                "other",
                MessageStatus::Read,
            ),
            (
                "pending",
                "PLAIN_AUDIO",
                MediaStatus::Pending,
                "other",
                MessageStatus::Read,
            ),
            (
                "anchor",
                "PLAIN_AUDIO",
                MediaStatus::Done,
                "other",
                MessageStatus::Delivered,
            ),
            (
                "last",
                "ENCRYPTED_AUDIO",
                MediaStatus::Read,
                "other",
                MessageStatus::Sent,
            ),
        ] {
            dao.insert_message(&Message {
                category: category.into(),
                media_status,
                user_id: sender.into(),
                status,
                media_url: Some(format!("/{id}.ogg")),
                created_at: time,
                ..message(id)
            })
            .await
            .unwrap();
        }
        let audio = dao
            .list_audio_items_around("conversation", "anchor", 1, 1)
            .await
            .unwrap();
        assert_eq!(
            audio
                .iter()
                .map(|item| item.message_id.as_str())
                .collect::<Vec<_>>(),
            ["first", "anchor", "last"]
        );
        assert!(dao
            .list_audio_items_around("conversation", "pending", 1, 1)
            .await
            .unwrap()
            .is_empty());
        assert!(dao
            .list_audio_items_around("other-conversation", "anchor", 1, 1)
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            dao.first_unread_message_id("conversation", "me")
                .await
                .unwrap()
                .as_deref(),
            Some("first")
        );
        dao.mark_conversation_read("conversation", "me")
            .await
            .unwrap();
        assert_eq!(
            dao.first_unread_message_id("conversation", "me")
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn stores_attachment_material_as_base64_and_decodes_on_read() {
        let (_directory, database) = test_database().await;
        let message = Message {
            media_key: Some((0_u8..64).collect()),
            media_digest: Some((0_u8..32).collect()),
            ..message("attachment")
        };
        database.message_dao.insert_message(&message).await.unwrap();

        let stored: (String, String) = crate::db::query_as(
            "SELECT media_key, media_digest FROM messages WHERE message_id = 'attachment'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert_eq!(
            stored.0,
            Base64::encode_string(message.media_key.as_deref().unwrap())
        );
        assert_eq!(
            stored.1,
            Base64::encode_string(message.media_digest.as_deref().unwrap())
        );

        let loaded = database
            .message_dao
            .find_message_by_id(&message.message_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.media_key, message.media_key);
        assert_eq!(loaded.media_digest, message.media_digest);
    }

    #[tokio::test]
    async fn nonmembers_and_quit_groups_reject_every_outgoing_insert_without_changing_draft_or_projection(
    ) {
        for quit in [false, true] {
            for kind in ["message", "pending", "transcript", "batch"] {
                let (_directory, database) = test_database().await;
                crate::db::query("UPDATE conversations SET category = 'GROUP', status = ?, draft = 'unfinished', last_message_id = 'previous', last_message_created_at = 7")
                    .bind(if quit { 3 } else { 2 }).execute(&database.message_dao.0).await.unwrap();
                if quit {
                    crate::db::query("INSERT INTO participants (conversation_id, user_id, role, created_at) VALUES ('conversation', 'sender', '', 0)")
                        .execute(&database.message_dao.0).await.unwrap();
                }
                assert!(!database
                    .conversation_dao
                    .can_send("conversation", "sender")
                    .await
                    .unwrap());
                let outgoing = message("denied");
                let job =
                    Job::create_sending_job("denied", "conversation", None, None, false, false, 0);
                let transcripts = [serde_json::from_value::<TranscriptMessage>(
                    serde_json::json!({
                        "transcript_id": "denied", "message_id": "child", "category": PLAIN_TEXT,
                        "created_at": Utc::now().to_rfc3339(), "content": "hello"
                    }),
                )
                .unwrap()];
                let result = match kind {
                    "pending" => {
                        database
                            .message_dao
                            .insert_pending_outgoing_message(&outgoing)
                            .await
                    }
                    "transcript" => {
                        database
                            .message_dao
                            .insert_outgoing_message_with_transcripts(&outgoing, &job, &transcripts)
                            .await
                    }
                    "batch" => {
                        database
                            .message_dao
                            .insert_outgoing_batch_with_transcripts(&[(
                                &outgoing,
                                &job,
                                Some(&transcripts),
                            )])
                            .await
                    }
                    _ => {
                        database
                            .message_dao
                            .insert_outgoing_message(&outgoing, &job)
                            .await
                    }
                };
                assert!(
                    matches!(result, Err(Error::PermissionDenied)),
                    "{kind}, quit={quit}"
                );
                let counts: (i64, i64, i64) = crate::db::query_as("SELECT (SELECT COUNT(*) FROM messages), (SELECT COUNT(*) FROM jobs), (SELECT COUNT(*) FROM transcript_messages)")
                    .fetch_one(&database.message_dao.0).await.unwrap();
                assert_eq!(counts, (0, 0, 0), "{kind}, quit={quit}");
                let projection: (String, String, i64) = crate::db::query_as("SELECT draft, last_message_id, last_message_created_at FROM conversations WHERE conversation_id = 'conversation'")
                    .fetch_one(&database.message_dao.0).await.unwrap();
                assert_eq!(projection, ("unfinished".into(), "previous".into(), 7));
                database
                    .message_dao
                    .insert_message(&Message {
                        user_id: "peer".into(),
                        status: MessageStatus::Delivered,
                        ..message("incoming")
                    })
                    .await
                    .unwrap();
                assert!(database
                    .message_dao
                    .find_message_by_id(&"incoming".into())
                    .await
                    .unwrap()
                    .is_some());
            }
        }
    }

    #[tokio::test]
    async fn contacts_and_existing_group_members_can_send_without_requiring_success_status() {
        for category in ["CONTACT", "GROUP"] {
            for status in [0, 1, 2, 3] {
                if category == "GROUP" && status == 3 {
                    continue;
                }
                let (_directory, database) = test_database().await;
                crate::db::query("UPDATE conversations SET category = ?, status = ?")
                    .bind(category)
                    .bind(status)
                    .execute(&database.message_dao.0)
                    .await
                    .unwrap();
                if category == "GROUP" {
                    crate::db::query("INSERT INTO participants (conversation_id, user_id, role, created_at) VALUES ('conversation', 'sender', '', 0)")
                        .execute(&database.message_dao.0).await.unwrap();
                }
                assert!(database
                    .conversation_dao
                    .can_send("conversation", "sender")
                    .await
                    .unwrap());
                let outgoing = message("allowed");
                let job =
                    Job::create_sending_job("allowed", "conversation", None, None, false, false, 0);
                database
                    .message_dao
                    .insert_outgoing_message(&outgoing, &job)
                    .await
                    .unwrap();
                let counts: (i64, i64) = crate::db::query_as(
                    "SELECT (SELECT COUNT(*) FROM messages), (SELECT COUNT(*) FROM jobs)",
                )
                .fetch_one(&database.message_dao.0)
                .await
                .unwrap();
                assert_eq!(counts, (1, 1));
            }
        }
    }

    #[tokio::test]
    async fn membership_removed_after_preflight_is_rechecked_inside_outgoing_transaction() {
        let (_directory, database) = test_database().await;
        crate::db::query(
            "UPDATE conversations SET category = 'GROUP', status = 2, draft = 'unfinished'",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        crate::db::query("INSERT INTO participants (conversation_id, user_id, role, created_at) VALUES ('conversation', 'sender', '', 0)")
            .execute(&database.message_dao.0).await.unwrap();
        assert!(database
            .conversation_dao
            .can_send("conversation", "sender")
            .await
            .unwrap());
        let mut transaction = database
            .message_dao
            .0
            .begin_with("BEGIN IMMEDIATE")
            .await
            .unwrap();
        crate::db::query("DELETE FROM participants WHERE conversation_id = 'conversation' AND user_id = 'sender'")
            .execute(&mut *transaction).await.unwrap();
        let outgoing = message("denied");
        let job = Job::create_sending_job("denied", "conversation", None, None, false, false, 0);
        assert!(matches!(
            MessageDao::insert_outgoing_message_with(&mut transaction, &outgoing, &job).await,
            Err(Error::PermissionDenied)
        ));
        transaction.commit().await.unwrap();
        let counts: (i64, i64, String) = crate::db::query_as("SELECT (SELECT COUNT(*) FROM messages), (SELECT COUNT(*) FROM jobs), draft FROM conversations WHERE conversation_id = 'conversation'")
            .fetch_one(&database.message_dao.0).await.unwrap();
        assert_eq!(counts, (0, 0, "unfinished".into()));
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
        let persisted_job: i64 =
            crate::db::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
                .bind(&job.job_id)
                .fetch_one(&database.message_dao.0)
                .await
                .unwrap();
        assert_eq!(persisted_job, 1);
        let last_message_id: Option<String> = crate::db::query_scalar(
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
    async fn second_batch_message_or_job_failure_rolls_back_all_rows_and_conversation_projection() {
        for failing_table in ["messages", "jobs"] {
            let (_directory, database) = test_database().await;
            crate::db::query("UPDATE conversations SET draft = 'unfinished', last_message_id = 'previous', last_message_created_at = 7")
                .execute(&database.message_dao.0).await.unwrap();
            let first = message("first");
            let second = message("second");
            let mut first_job =
                Job::create_sending_job("first", "conversation", None, None, false, false, 0);
            first_job.job_id = "first-job".into();
            let mut second_job =
                Job::create_sending_job("second", "conversation", None, None, false, false, 0);
            second_job.job_id = "second-job".into();
            let transcripts = [
                serde_json::from_value::<TranscriptMessage>(serde_json::json!({
                    "transcript_id": "first", "message_id": "child", "category": PLAIN_TEXT,
                    "created_at": Utc::now().to_rfc3339(), "content": "hello"
                }))
                .unwrap(),
            ];
            let trigger = if failing_table == "messages" {
                "CREATE TRIGGER reject_second BEFORE INSERT ON messages WHEN NEW.message_id = 'second' BEGIN SELECT RAISE(ABORT, 'rejected'); END"
            } else {
                "CREATE TRIGGER reject_second BEFORE INSERT ON jobs WHEN NEW.job_id = 'second-job' BEGIN SELECT RAISE(ABORT, 'rejected'); END"
            };
            crate::db::query(trigger)
                .execute(&database.message_dao.0)
                .await
                .unwrap();
            let batch = [
                (&first, &first_job, Some(transcripts.as_slice())),
                (&second, &second_job, None),
            ];
            assert!(database
                .message_dao
                .insert_outgoing_batch_with_transcripts(&batch)
                .await
                .is_err());
            for (table, sql) in [
                ("messages", "SELECT COUNT(*) FROM messages"),
                ("jobs", "SELECT COUNT(*) FROM jobs"),
                (
                    "transcript_messages",
                    "SELECT COUNT(*) FROM transcript_messages",
                ),
            ] {
                let count: i64 = crate::db::query_scalar(sql)
                    .fetch_one(&database.message_dao.0)
                    .await
                    .unwrap();
                assert_eq!(
                    count, 0,
                    "{table} must roll back when {failing_table} rejects the second item"
                );
            }
            let projection: (String, String, i64) = crate::db::query_as("SELECT draft, last_message_id, last_message_created_at FROM conversations WHERE conversation_id = 'conversation'")
                .fetch_one(&database.message_dao.0).await.unwrap();
            assert_eq!(projection, ("unfinished".into(), "previous".into(), 7));
            crate::db::query("DROP TRIGGER reject_second")
                .execute(&database.message_dao.0)
                .await
                .unwrap();
            database
                .message_dao
                .insert_outgoing_batch_with_transcripts(&batch)
                .await
                .unwrap();
            assert_eq!(database.job_dao.sending_jobs().await.unwrap().len(), 2);
            assert_eq!(
                database
                    .transcript_message_dao
                    .find_by_transcript_id("first")
                    .await
                    .unwrap()
                    .len(),
                1
            );
            let projection: (String, String) = crate::db::query_as("SELECT draft, last_message_id FROM conversations WHERE conversation_id = 'conversation'")
                .fetch_one(&database.message_dao.0).await.unwrap();
            assert_eq!(projection, (String::new(), "second".into()));
        }
    }

    #[tokio::test]
    async fn reopened_database_recovers_only_current_accounts_interrupted_outgoing_attachments() {
        let (directory, database) = test_database().await;
        let cases = [
            (
                "image",
                "me",
                MessageStatus::Sending,
                "SIGNAL_IMAGE",
                MediaStatus::Pending,
                MediaStatus::Canceled,
            ),
            (
                "video",
                "me",
                MessageStatus::Sending,
                "ENCRYPTED_VIDEO",
                MediaStatus::Pending,
                MediaStatus::Canceled,
            ),
            (
                "audio",
                "me",
                MessageStatus::Sending,
                "PLAIN_AUDIO",
                MediaStatus::Pending,
                MediaStatus::Canceled,
            ),
            (
                "data",
                "me",
                MessageStatus::Sending,
                "SIGNAL_DATA",
                MediaStatus::Pending,
                MediaStatus::Canceled,
            ),
            (
                "ready",
                "me",
                MessageStatus::Sending,
                "SIGNAL_IMAGE",
                MediaStatus::Done,
                MediaStatus::Done,
            ),
            (
                "peer",
                "peer",
                MessageStatus::Sending,
                "SIGNAL_IMAGE",
                MediaStatus::Pending,
                MediaStatus::Pending,
            ),
            (
                "sent",
                "me",
                MessageStatus::Sent,
                "SIGNAL_IMAGE",
                MediaStatus::Pending,
                MediaStatus::Pending,
            ),
            (
                "text",
                "me",
                MessageStatus::Sending,
                "SIGNAL_TEXT",
                MediaStatus::Pending,
                MediaStatus::Pending,
            ),
        ];
        let mut job_ids = Vec::new();
        for (id, sender, status, category, before, _) in cases.clone() {
            let outgoing = Message {
                user_id: sender.into(),
                status,
                category: category.into(),
                media_status: before,
                ..message(id)
            };
            let job = Job::create_sending_job(id, "conversation", None, None, false, false, 0);
            database
                .message_dao
                .insert_outgoing_message(&outgoing, &job)
                .await
                .unwrap();
            job_ids.push(job.job_id);
        }
        database.close().await;
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        database
            .message_dao
            .recover_interrupted_uploads("me")
            .await
            .unwrap();
        for (id, _, status, _, _, expected) in cases {
            let stored = database
                .message_dao
                .find_message_by_id(&id.to_string())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                stored.media_status, expected,
                "unexpected recovery for {id}"
            );
            assert_eq!(stored.status, status);
        }
        let stored_job_ids = database
            .job_dao
            .sending_jobs()
            .await
            .unwrap()
            .into_iter()
            .map(|job| job.job_id)
            .collect::<HashSet<_>>();
        assert_eq!(stored_job_ids, job_ids.into_iter().collect());
    }

    #[tokio::test]
    async fn rolls_back_transcripts_when_outgoing_message_fails() {
        let (_directory, database) = test_database().await;
        crate::db::query(
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
        let count: i64 = crate::db::query_scalar(
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
        crate::db::query(
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
        let messages: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE message_id IN ('first', 'second')",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let fts: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM fts.messages_metas WHERE message_id IN ('first', 'second')",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert_eq!((messages, fts), (2, 2));
    }

    #[tokio::test]
    async fn clears_only_the_selected_conversation_and_related_indexes() {
        let (_directory, database) = test_database().await;
        crate::db::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('other-conversation', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
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
        crate::db::query(
            "INSERT INTO pin_messages (message_id, conversation_id, created_at) \
             VALUES ('first', 'conversation', CURRENT_TIMESTAMP)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        crate::db::query(
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

        let selected_messages: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let other_messages: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = 'other-conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let selected_fts: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM fts.messages_metas WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let pin_count: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM pin_messages WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let expired_count: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM expired_messages WHERE message_id = 'second'",
        )
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
    async fn queues_recall_jobs_without_mutating_messages() {
        let (_directory, database) = test_database().await;
        for message_id in ["first", "second"] {
            database
                .message_dao
                .insert_message(&message(message_id))
                .await
                .unwrap();
        }
        let ids = ["first".to_string(), "second".to_string()];
        let jobs = ids
            .iter()
            .map(|message_id| Job::create_send_recall_job("conversation", message_id))
            .collect::<Vec<_>>();

        database
            .message_dao
            .insert_recall_jobs("conversation", &ids, &jobs)
            .await
            .unwrap();
        let recalled: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM messages WHERE message_id IN ('first', 'second') \
             AND category = 'MESSAGE_RECALL'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!((recalled, jobs), (0, 2));
    }

    #[tokio::test]
    async fn rolls_back_recall_jobs_when_any_message_is_missing() {
        let (_directory, database) = test_database().await;
        database
            .message_dao
            .insert_message(&message("first"))
            .await
            .unwrap();
        let ids = ["first".to_string(), "missing".to_string()];
        let jobs = ids
            .iter()
            .map(|message_id| Job::create_send_recall_job("conversation", message_id))
            .collect::<Vec<_>>();

        assert!(database
            .message_dao
            .insert_recall_jobs("conversation", &ids, &jobs)
            .await
            .is_err());
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(jobs, 0);
    }

    #[tokio::test]
    async fn duplicate_recall_request_does_not_reset_acknowledged_job() {
        let (_directory, database) = test_database().await;
        database
            .message_dao
            .insert_message(&message("message"))
            .await
            .unwrap();
        let ids = ["message".to_string()];
        let jobs = [Job::create_send_recall_job("conversation", "message")];
        database
            .message_dao
            .insert_recall_jobs("conversation", &ids, &jobs)
            .await
            .unwrap();
        database
            .job_dao
            .reschedule_job(&jobs[0].job_id, Utc::now().naive_utc(), 1)
            .await
            .unwrap();

        database
            .message_dao
            .insert_recall_jobs("conversation", &ids, &jobs)
            .await
            .unwrap();

        let run_count: i32 = crate::db::query_scalar("SELECT run_count FROM jobs WHERE job_id = ?")
            .bind(&jobs[0].job_id)
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(run_count, 1);
    }

    #[tokio::test]
    async fn rolls_back_pin_job_when_pin_mutation_fails() {
        let (_directory, database) = test_database().await;
        database
            .message_dao
            .insert_message(&message("message"))
            .await
            .unwrap();
        crate::db::query(
            "CREATE TRIGGER reject_pin BEFORE INSERT ON pin_messages \
             BEGIN SELECT RAISE(ABORT, 'rejected'); END",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let job = Job::create_send_pin_job("conversation", "payload");
        let pin_event = pin_event("pin-event", "message");

        assert!(database
            .message_dao
            .set_message_pinned_with_job(
                "conversation",
                "message",
                true,
                Utc::now(),
                Some(&pin_event),
                &job,
            )
            .await
            .is_err());
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        let pinned: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM pin_messages")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        let pin_events: i64 =
            crate::db::query_scalar("SELECT COUNT(*) FROM messages WHERE category = 'MESSAGE_PIN'")
                .fetch_one(&database.message_dao.0)
                .await
                .unwrap();
        assert_eq!(jobs, 0);
        assert_eq!(pinned, 0);
        assert_eq!(pin_events, 0);
    }

    #[tokio::test]
    async fn pins_message_with_local_event_and_job_atomically() {
        let (_directory, database) = test_database().await;
        database
            .message_dao
            .insert_message(&message("message"))
            .await
            .unwrap();
        let job = Job::create_send_pin_job("conversation", "payload");
        let pin_event = pin_event("pin-event", "message");
        let created_at = Utc::now();

        database
            .message_dao
            .set_message_pinned_with_job(
                "conversation",
                "message",
                true,
                created_at,
                Some(&pin_event),
                &job,
            )
            .await
            .unwrap();

        let pinned: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM pin_messages WHERE message_id = 'message'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let stored_event = database
            .message_dao
            .find_message_by_id(&"pin-event".to_string())
            .await
            .unwrap()
            .unwrap();
        let last_message_id: Option<String> = crate::db::query_scalar(
            "SELECT last_message_id FROM conversations WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();

        assert_eq!(pinned, 1);
        assert_eq!(stored_event.category, MESSAGE_PIN);
        assert_eq!(stored_event.quote_message_id.as_deref(), Some("message"));
        assert_eq!(last_message_id.as_deref(), Some("pin-event"));
        assert_eq!(jobs, 1);
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
        let job_count: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs WHERE job_id = ?")
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
    async fn time_range_pages_include_start_exclude_end_and_preserve_timestamp_ties() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        let base = chrono::DateTime::from_timestamp_millis(1000)
            .unwrap()
            .naive_utc();
        for (id, offset) in [
            ("before", -1),
            ("start", 0),
            ("tie1", 1),
            ("tie2", 1),
            ("end", 2),
        ] {
            dao.insert_message(&Message {
                message_id: id.into(),
                created_at: base + chrono::Duration::milliseconds(offset),
                ..message(id)
            })
            .await
            .unwrap();
        }
        let range = (Some(1000), Some(1002));
        let first = dao
            .list_items_in_range("conversation", None, None, 1, range)
            .await
            .unwrap();
        assert_eq!(first[0].message_id, "tie2");
        let second = dao
            .list_items_in_range("conversation", Some(1001), Some("tie2"), 10, range)
            .await
            .unwrap();
        assert_eq!(
            second
                .iter()
                .map(|item| item.message_id.as_str())
                .collect::<Vec<_>>(),
            ["tie1", "start"]
        );
        assert!(dao
            .list_items_in_range("conversation", Some(1000), Some("start"), 10, range)
            .await
            .unwrap()
            .is_empty());
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
        let cursor_created_at = cursor.created_at.and_utc().timestamp_millis();
        let cursor_message_id = cursor.message_id.clone();
        crate::db::query("DELETE FROM messages WHERE message_id = ?")
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
    async fn lists_sender_participant_role_for_recall_policy() {
        let (_directory, database) = test_database().await;
        database
            .message_dao
            .insert_message(&message("message"))
            .await
            .unwrap();
        crate::db::query(
            "INSERT INTO participants (conversation_id, user_id, role, created_at) \
             VALUES ('conversation', 'sender', 'ADMIN', 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();

        let item = database
            .message_dao
            .list_items("conversation", None, None, 1)
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(item.sender_participant_id.as_deref(), Some("sender"));
        assert_eq!(item.sender_role.as_deref(), Some("ADMIN"));
    }

    #[tokio::test]
    async fn message_window_queries_use_row_id_ordering() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        let created_at = Utc::now().naive_utc();
        for message_id in ["first", "center", "last"] {
            dao.insert_message(&Message {
                message_id: message_id.to_string(),
                created_at,
                ..message(message_id)
            })
            .await
            .unwrap();
        }

        let center = dao.message_order_info("center").await.unwrap().unwrap();
        assert_eq!(
            dao.message_ids_before("conversation", &center, 10)
                .await
                .unwrap(),
            ["first"]
        );
        assert_eq!(
            dao.message_ids_after("conversation", &center, 10)
                .await
                .unwrap(),
            ["last"]
        );
        let around = dao
            .list_items_around("conversation", "center", 1, 1)
            .await
            .unwrap();
        assert_eq!(
            around
                .iter()
                .map(|message| message.message_id.as_str())
                .collect::<Vec<_>>(),
            ["first", "center", "last"]
        );
        assert!(dao
            .list_items_around("conversation", "missing", 1, 1)
            .await
            .unwrap()
            .is_empty());
        let items = dao
            .list_items_by_ids(&["last".to_string(), "first".to_string()])
            .await
            .unwrap();
        assert_eq!(items.len(), 2);

        let plan = crate::db::query_as::<(i64, i64, i64, String)>(
            "EXPLAIN QUERY PLAN SELECT message_id FROM messages \
             WHERE conversation_id = ? ORDER BY created_at DESC, rowid DESC LIMIT 60",
        )
        .bind("conversation")
        .fetch_all(&dao.0)
        .await
        .unwrap();
        assert!(plan.iter().any(|(_, _, _, detail)| {
            detail.contains("index_messages_conversation_id_created_at")
        }));
    }

    #[tokio::test]
    async fn finds_failed_messages_with_the_filtering_index() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        let base = Utc::now().naive_utc();
        for (message_id, user_id, status, offset) in [
            ("old-failed", "sender", MessageStatus::Failed, 0),
            ("sent", "sender", MessageStatus::Sent, 1),
            ("other-user", "other", MessageStatus::Failed, 2),
            ("new-failed", "sender", MessageStatus::Failed, 3),
        ] {
            dao.insert_message(&Message {
                message_id: message_id.to_string(),
                user_id: user_id.to_string(),
                status,
                created_at: base + chrono::Duration::seconds(offset),
                ..message(message_id)
            })
            .await
            .unwrap();
        }

        assert_eq!(
            dao.find_failed_message("conversation", "sender")
                .await
                .unwrap(),
            ["new-failed", "old-failed"]
        );
        let plan = crate::db::query_as::<(i64, i64, i64, String)>(
            "EXPLAIN QUERY PLAN SELECT message_id FROM messages \
             INDEXED BY index_message_conversation_id_status_user_id \
             WHERE conversation_id = ? AND user_id = ? AND status = ? \
             ORDER BY created_at DESC LIMIT 1000",
        )
        .bind("conversation")
        .bind("sender")
        .bind(MessageStatus::Failed)
        .fetch_all(&dao.0)
        .await
        .unwrap();
        assert!(plan.iter().any(|(_, _, _, detail)| {
            detail.contains("index_message_conversation_id_status_user_id")
        }));
    }

    #[tokio::test]
    async fn lists_transferred_messages_with_missing_media_status() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        dao.insert_message(&message("empty")).await.unwrap();
        dao.insert_message(&message("null")).await.unwrap();
        crate::db::query("UPDATE messages SET media_status = '' WHERE message_id = 'empty'")
            .execute(&dao.0)
            .await
            .unwrap();
        crate::db::query("UPDATE messages SET media_status = NULL WHERE message_id = 'null'")
            .execute(&dao.0)
            .await
            .unwrap();

        let items = dao.list_items("conversation", None, None, 2).await.unwrap();
        let message = dao
            .find_message_by_id(&"null".to_string())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.media_status == MediaStatus::Canceled));
        assert_eq!(message.media_status, MediaStatus::Canceled);
    }

    #[tokio::test]
    async fn lists_pinned_messages_in_pin_order_with_full_projection() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        crate::db::query(
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
        crate::db::query(
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
            crate::db::query(
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
        assert_eq!(dao.pinned_message_count("conversation").await.unwrap(), 3);
        let page = dao
            .list_pinned_items_page("conversation", 1, 1)
            .await
            .unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].message_id, "beta");
        assert_eq!(
            dao.pinned_message_offset("conversation", "beta")
                .await
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            dao.pinned_message_offset("other", "beta").await.unwrap(),
            None
        );
        assert_eq!(
            dao.pinned_message_offset("conversation", "absent")
                .await
                .unwrap(),
            None
        );
        assert!(dao
            .list_pinned_items_page("conversation", 3, 50)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn unpin_all_rolls_back_every_chunk_when_a_later_job_fails_then_retries_atomically() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        for index in 0..205 {
            let id = format!("pin-{index:03}");
            dao.insert_message(&message(&id)).await.unwrap();
            crate::db::query("INSERT INTO pin_messages (message_id,conversation_id,created_at) VALUES (?,'conversation',0)")
                .bind(id).execute(&dao.0).await.unwrap();
        }
        crate::db::query("INSERT INTO conversations (conversation_id,created_at,status,category) VALUES ('other',0,0,'CONTACT')")
            .execute(&dao.0).await.unwrap();
        let mut other = message("other-pin");
        other.conversation_id = "other".into();
        dao.insert_message(&other).await.unwrap();
        crate::db::query("INSERT INTO pin_messages (message_id,conversation_id,created_at) VALUES ('other-pin','other',0)").execute(&dao.0).await.unwrap();
        assert_eq!(
            dao.list_pinned_items_page("conversation", 0, 1000)
                .await
                .unwrap()
                .len(),
            200
        );
        crate::db::query("CREATE TRIGGER reject_second_unpin BEFORE INSERT ON jobs WHEN (SELECT COUNT(*) FROM jobs)>0 BEGIN SELECT RAISE(ABORT,'reject second chunk'); END")
            .execute(&dao.0).await.unwrap();
        assert!(dao
            .unpin_all_with_jobs("conversation", "sender")
            .await
            .is_err());
        assert_eq!(dao.pinned_message_count("conversation").await.unwrap(), 205);
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&dao.0)
            .await
            .unwrap();
        assert_eq!(jobs, 0);
        crate::db::query("DROP TRIGGER reject_second_unpin")
            .execute(&dao.0)
            .await
            .unwrap();
        dao.unpin_all_with_jobs("conversation", "sender")
            .await
            .unwrap();
        assert_eq!(dao.pinned_message_count("conversation").await.unwrap(), 0);
        assert_eq!(dao.pinned_message_count("other").await.unwrap(), 1);
        let payloads: Vec<String> =
            crate::db::query_scalar("SELECT blaze_message FROM jobs WHERE action = ?")
                .bind(sdk::PIN_MESSAGE)
                .fetch_all(&dao.0)
                .await
                .unwrap();
        assert_eq!(payloads.len(), 3);
        let mut unpinned = HashSet::new();
        for payload in payloads {
            let sdk::PinMessagePayload::Unpin(ids) = serde_json::from_str(&payload).unwrap() else {
                panic!("expected unpin job");
            };
            assert!(ids.len() <= 100);
            for id in ids {
                assert!(unpinned.insert(id));
            }
        }
        assert_eq!(unpinned.len(), 205);
        dao.unpin_all_with_jobs("conversation", "sender")
            .await
            .unwrap();
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&dao.0)
            .await
            .unwrap();
        assert_eq!(jobs, 3);
        database.close().await;
    }

    #[tokio::test]
    async fn unpin_all_rechecks_group_membership_before_enqueuing_or_removing_pins() {
        let (_directory, database) = test_database().await;
        let dao = &database.message_dao;
        dao.insert_message(&message("pinned")).await.unwrap();
        crate::db::query("INSERT INTO pin_messages (message_id,conversation_id,created_at) VALUES ('pinned','conversation',0)").execute(&dao.0).await.unwrap();
        crate::db::query(
            "UPDATE conversations SET category='GROUP' WHERE conversation_id='conversation'",
        )
        .execute(&dao.0)
        .await
        .unwrap();
        assert!(matches!(
            dao.unpin_all_with_jobs("conversation", "removed-member")
                .await,
            Err(Error::PermissionDenied)
        ));
        assert_eq!(dao.pinned_message_count("conversation").await.unwrap(), 1);
        let jobs: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&dao.0)
            .await
            .unwrap();
        assert_eq!(jobs, 0);
        crate::db::query("INSERT INTO participants (conversation_id,user_id,role,created_at) VALUES ('conversation','active-member','',0)")
            .execute(&dao.0).await.unwrap();
        assert!(dao
            .can_unpin_messages("conversation", "active-member")
            .await
            .unwrap());
        crate::db::query("UPDATE conversations SET status=? WHERE conversation_id='conversation'")
            .bind(ConversationStatus::QUIT.0)
            .execute(&dao.0)
            .await
            .unwrap();
        assert!(!dao
            .can_unpin_messages("conversation", "active-member")
            .await
            .unwrap());
        assert!(matches!(
            dao.unpin_all_with_jobs("conversation", "active-member")
                .await,
            Err(Error::PermissionDenied)
        ));
        assert_eq!(dao.pinned_message_count("conversation").await.unwrap(), 1);
        database.close().await;
    }

    #[tokio::test]
    async fn lists_image_messages_around_cursor_in_stable_order() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        crate::db::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('other', 0, 0)",
        )
        .execute(&dao.0)
        .await
        .unwrap();
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
        crate::db::query(
            "INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)",
        )
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
        assert_eq!(
            unread.into_iter().collect::<HashSet<_>>(),
            HashSet::from(["sent".to_string(), "delivered".to_string(),])
        );
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

        assert_eq!(
            database
                .message_dao
                .mark_conversation_read("conversation", "me")
                .await
                .unwrap(),
            (true, true)
        );

        let status: MessageStatus =
            crate::db::query_scalar("SELECT status FROM messages WHERE message_id = 'incoming'")
                .fetch_one(&database.message_dao.0)
                .await
                .unwrap();
        assert_eq!(status, MessageStatus::Read);
        let expire_at: i64 = crate::db::query_scalar(
            "SELECT expire_at FROM expired_messages WHERE message_id = 'incoming'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert!(expire_at >= Utc::now().timestamp() + 59);
        let actions: Vec<String> =
            crate::db::query_scalar("SELECT action FROM jobs ORDER BY action")
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
        assert_eq!(
            database
                .message_dao
                .mark_conversation_read("conversation", "me")
                .await
                .unwrap(),
            (false, false)
        );
    }

    #[tokio::test]
    async fn mark_read_waits_for_a_concurrent_writer_without_blocking_readers() {
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

        let mut writer = database
            .message_dao
            .0
            .begin_with("BEGIN IMMEDIATE")
            .await
            .unwrap();
        crate::db::query("UPDATE conversations SET name = 'writer' WHERE conversation_id = ?")
            .bind("conversation")
            .execute(&mut *writer)
            .await
            .unwrap();

        let dao = database.message_dao.clone();
        let mark_read =
            tokio::spawn(async move { dao.mark_conversation_read("conversation", "me").await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!mark_read.is_finished());

        let message_count: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM messages")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(message_count, 1);

        writer.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), mark_read)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            (true, true)
        );
    }

    #[tokio::test]
    async fn mark_read_noop_does_not_wait_for_a_concurrent_writer() {
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
        crate::db::query(
            "UPDATE conversations SET unseen_message_count = 0 WHERE conversation_id = ?",
        )
        .bind("conversation")
        .execute(&database.message_dao.0)
        .await
        .unwrap();

        let mut writer = database
            .message_dao
            .0
            .begin_with("BEGIN IMMEDIATE")
            .await
            .unwrap();
        crate::db::query("UPDATE conversations SET name = 'writer' WHERE conversation_id = ?")
            .bind("conversation")
            .execute(&mut *writer)
            .await
            .unwrap();

        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(1),
                database
                    .message_dao
                    .mark_conversation_read("conversation", "me"),
            )
            .await
            .unwrap()
            .unwrap(),
            (false, false)
        );
        writer.rollback().await.unwrap();
    }

    #[tokio::test]
    async fn clears_stale_unseen_count_without_unread_messages() {
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
        database
            .message_dao
            .insert_message(&Message {
                message_id: "read".into(),
                user_id: "other".into(),
                status: MessageStatus::Read,
                ..message("read")
            })
            .await
            .unwrap();
        crate::db::query(
            "UPDATE conversations SET unseen_message_count = 3 \
             WHERE conversation_id = 'conversation'",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();

        assert_eq!(
            database
                .message_dao
                .mark_conversation_read("conversation", "me")
                .await
                .unwrap(),
            (false, true)
        );

        let state: (Option<String>, i64) = crate::db::query_as(
            "SELECT last_read_message_id, unseen_message_count FROM conversations \
             WHERE conversation_id = 'conversation'",
        )
        .fetch_one(&database.message_dao.0)
        .await
        .unwrap();
        assert_eq!(state, (Some("read".into()), 0));
        let job_count: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&database.message_dao.0)
            .await
            .unwrap();
        assert_eq!(job_count, 0);
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
        crate::db::query(
            "INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)",
        )
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
        let quote: serde_json::Value =
            serde_json::from_str(reply.quote_content.as_deref().unwrap()).unwrap();
        assert_eq!(quote["message_id"], "quoted");
        assert_eq!(quote["user_id"], "sender");
        assert_eq!(quote["user_full_name"], "Sender");
        assert!(quote.get("createdAt").is_some());
        assert!(quote.get("messageId").is_none());
        assert!(quote.get("userFullName").is_none());
    }

    #[tokio::test]
    async fn keeps_refreshed_quote_media_url_relative() {
        let (_directory, database) = test_database().await;
        let dao = database.message_dao;
        crate::db::query(
            "INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)",
        )
        .bind("sender")
        .bind("7000")
        .bind("Sender")
        .execute(&dao.0)
        .await
        .unwrap();

        let mut quoted = message("quoted");
        quoted.category = "PLAIN_IMAGE".to_string();
        quoted.media_url = Some("quoted.png".to_string());
        quoted.media_mime_type = Some("image/png".to_string());
        dao.insert_message(&quoted).await.unwrap();
        let mut reply = message("reply");
        reply.quote_message_id = Some("quoted".to_string());
        dao.insert_message(&reply).await.unwrap();

        dao.update_message_quote_if_need("conversation", "quoted")
            .await
            .unwrap();

        let reply = dao
            .find_message_by_id(&"reply".to_string())
            .await
            .unwrap()
            .unwrap();
        let quote: serde_json::Value =
            serde_json::from_str(reply.quote_content.as_deref().unwrap()).unwrap();
        assert_eq!(quote["media_url"], "quoted.png");
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
        crate::db::query(
            "INSERT INTO users (user_id, identity_number, full_name) VALUES (?, ?, ?)",
        )
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
        crate::db::query(
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
        let pin_count: i64 = crate::db::query_scalar(
            "SELECT COUNT(*) FROM pin_messages WHERE message_id = 'original'",
        )
        .fetch_one(&dao.0)
        .await
        .unwrap();
        assert_eq!(pin_count, 0);
        assert!(database
            .message_fts_dao
            .search(&crate::db::mixin::message_fts::MessageFtsSearch {
                query: "secret".into(),
                limit: 10,
                ..Default::default()
            })
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

crate::db::storage::row_mapping!(Message {
    message_id: "message_id",
    conversation_id: "conversation_id",
    user_id: "user_id",
    category: "category",
    content: "content",
    media_url: "media_url",
    media_mime_type: "media_mime_type",
    media_size: "media_size",
    media_duration: "media_duration",
    media_width: "media_width",
    media_height: "media_height",
    media_hash: "media_hash",
    thumb_image: "thumb_image",
    media_key: "media_key",
    media_digest: "media_digest",
    media_status: "media_status",
    status: "status",
    created_at: "created_at" => crate::db::datetime::DatabaseDateTime,
    action: "action",
    participant_id: "participant_id",
    snapshot_id: "snapshot_id",
    hyperlink: "hyperlink",
    name: "name",
    album_id: "album_id",
    sticker_id: "sticker_id",
    shared_user_id: "shared_user_id",
    media_waveform: "media_waveform",
    quote_message_id: "quote_message_id",
    quote_content: "quote_content",
    thumb_url: "thumb_url",
    caption: "caption",
});

crate::db::storage::row_mapping!(MessageListItem {
    message_id: "message_id",
    conversation_id: "conversation_id",
    user_id: "user_id",
    sender_name: "sender_name",
    sender_identity_number: "sender_identity_number",
    sender_avatar_url: "sender_avatar_url",
    sender_is_verified: "sender_is_verified",
    sender_membership: "sender_membership",
    sender_relationship: "sender_relationship",
    sender_app_id: "sender_app_id",
    sender_is_scam: "sender_is_scam",
    sender_is_bot: "sender_is_bot",
    sender_participant_id: "sender_participant_id",
    sender_role: "sender_role",
    category: "category",
    content: "content",
    status: "status",
    created_at: "created_at" => crate::db::datetime::DatabaseDateTime,
    media_url: "media_url",
    media_mime_type: "media_mime_type",
    media_size: "media_size",
    media_duration: "media_duration",
    media_width: "media_width",
    media_height: "media_height",
    thumb_image: "thumb_image",
    media_status: "media_status",
    quote_message_id: "quote_message_id",
    quote_content: "quote_content",
    quote_user_membership: "quote_user_membership",
    caption: "caption",
    action: "action",
    participant_id: "participant_id",
    participant_full_name: "participant_full_name",
    snapshot_id: "snapshot_id",
    snapshot_type: "snapshot_type",
    snapshot_amount: "snapshot_amount",
    snapshot_memo: "snapshot_memo",
    snapshot_asset_id: "snapshot_asset_id",
    snapshot_asset_symbol: "snapshot_asset_symbol",
    snapshot_asset_icon_url: "snapshot_asset_icon_url",
    snapshot_chain_icon_url: "snapshot_chain_icon_url",
    snapshot_opponent_id: "snapshot_opponent_id",
    snapshot_transaction_hash: "snapshot_transaction_hash",
    snapshot_created_at: "snapshot_created_at",
    inscription_hash: "inscription_hash",
    inscription_collection_hash: "inscription_collection_hash",
    inscription_sequence: "inscription_sequence",
    inscription_content_type: "inscription_content_type",
    inscription_content_url: "inscription_content_url",
    inscription_name: "inscription_name",
    inscription_icon_url: "inscription_icon_url",
    hyperlink: "hyperlink",
    media_name: "media_name",
    album_id: "album_id",
    sticker_id: "sticker_id",
    shared_user_id: "shared_user_id",
    media_waveform: "media_waveform",
    thumb_url: "thumb_url",
    conversation_owner_id: "conversation_owner_id",
    conversation_category: "conversation_category",
    shared_user_full_name: "shared_user_full_name",
    shared_user_identity_number: "shared_user_identity_number",
    shared_user_avatar_url: "shared_user_avatar_url",
    shared_user_is_verified: "shared_user_is_verified",
    shared_user_membership: "shared_user_membership",
    shared_user_app_id: "shared_user_app_id",
    sticker_asset_url: "sticker_asset_url",
    sticker_asset_width: "sticker_asset_width",
    sticker_asset_height: "sticker_asset_height",
    sticker_asset_name: "sticker_asset_name",
    sticker_asset_type: "sticker_asset_type",
    mention_read: "mention_read",
    pinned: "pinned",
    expire_in: "expire_in",
});

crate::db::storage::row_mapping!(MessageOrderInfo {
    message_id: "message_id",
    row_id: "row_id",
    created_at: "created_at" => crate::db::datetime::DatabaseDateTime,
});

crate::db::storage::row_mapping!(ImageMessageItem {
    message_id: "message_id",
    created_at: "created_at" => crate::db::datetime::DatabaseDateTime,
    media_url: "media_url",
    media_mime_type: "media_mime_type",
    media_name: "media_name",
    thumb_image: "thumb_image",
    can_forward: "can_forward",
    user_id: "user_id",
    user_full_name: "user_full_name",
    user_identity_number: "user_identity_number",
    avatar_url: "avatar_url",
});

crate::db::storage::row_mapping!(QuoteMessage {
    message_id: "messageId",
    conversation_id: "conversationId",
    user_id: "userId",
    user_full_name: "userFullName",
    user_identity_number: "userIdentityNumber",
    app_id: "appId",
    category: "type",
    content: "content",
    created_at: "createdAt" => crate::db::datetime::DatabaseDateTime,
    status: "status",
    media_status: "mediaStatus",
    media_waveform: "mediaWaveform",
    media_name: "mediaName",
    media_mime_type: "mediaMimeType",
    media_size: "mediaSize",
    media_width: "mediaWidth",
    media_height: "mediaHeight",
    thumb_image: "thumbImage",
    thumb_url: "thumbUrl",
    media_url: "mediaUrl",
    media_duration: "mediaDuration",
    sticker_id: "stickerId",
    asset_url: "assetUrl",
    asset_width: "assetWidth",
    asset_height: "assetHeight",
    asset_name: "assetName",
    asset_type: "assetType",
    shared_user_id: "sharedUserId",
    shared_user_full_name: "sharedUserFullName",
    shared_user_identity_number: "sharedUserIdentityNumber",
    shared_user_avatar_url: "sharedUserAvatarUrl",
    shared_user_is_verified: "sharedUserIsVerified",
    shared_user_app_id: "sharedUserAppId",
});

crate::db::storage::row_mapping!(MiniMessageItem {
    message_id: "message_id",
    conversation_id: "conversation_id",
});

crate::db::storage::row_mapping!(NotificationMessageItem {
    row_id: "row_id",
    message_id: "message_id",
    conversation_id: "conversation_id",
    user_id: "user_id",
    sender_name: "sender_name",
    action: "action",
    participant_id: "participant_id",
    participant_name: "participant_name",
    category: "category",
    status: "status",
    content: "content",
    quote_content: "quote_content",
    created_at: "created_at" => crate::db::datetime::DatabaseDateTime,
    conversation_name: "conversation_name",
    conversation_category: "conversation_category",
    is_muted: "is_muted",
});
