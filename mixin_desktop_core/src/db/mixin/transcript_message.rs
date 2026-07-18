use chrono::{DateTime, Utc};
use sdk::blaze_message::MessageStatus;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::db::mixin::message::MediaStatus;
use crate::db::Error;

const TRANSCRIPT_SELECT_COLUMNS: &str = r#"
    transcript_id, message_id, user_id, user_full_name, category, created_at,
    content, media_url, media_name, media_size, media_width, media_height,
    media_mime_type, media_duration, media_status, media_waveform, thumb_image,
    thumb_url, media_key, media_digest,
    CASE WHEN typeof(media_created_at) = 'integer'
         THEN strftime('%Y-%m-%dT%H:%M:%fZ', media_created_at / 1000.0, 'unixepoch')
         ELSE media_created_at END AS media_created_at,
    sticker_id, shared_user_id, mentions, quote_id, quote_content, caption
"#;

#[derive(Clone)]
pub struct TranscriptMessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Clone, Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct TranscriptMessage {
    pub transcript_id: String,
    pub message_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub user_full_name: Option<String>,
    pub category: String,
    #[serde(
        deserialize_with = "deserialize_datetime",
        serialize_with = "serialize_datetime"
    )]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub media_url: Option<String>,
    #[serde(default)]
    pub media_name: Option<String>,
    #[serde(default)]
    pub media_size: Option<i64>,
    #[serde(default)]
    pub media_width: Option<i32>,
    #[serde(default)]
    pub media_height: Option<i32>,
    #[serde(default)]
    pub media_mime_type: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_string",
        serialize_with = "serialize_optional_number"
    )]
    pub media_duration: Option<String>,
    #[serde(default, skip_serializing)]
    pub media_status: Option<MediaStatus>,
    #[serde(default)]
    pub media_waveform: Option<String>,
    #[serde(default)]
    pub thumb_image: Option<String>,
    #[serde(default)]
    pub thumb_url: Option<String>,
    #[serde(default)]
    pub media_key: Option<String>,
    #[serde(default)]
    pub media_digest: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_datetime",
        serialize_with = "serialize_optional_datetime"
    )]
    pub media_created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub sticker_id: Option<String>,
    #[serde(default)]
    pub shared_user_id: Option<String>,
    #[serde(default)]
    pub mentions: Option<String>,
    #[serde(default)]
    pub quote_id: Option<String>,
    #[serde(default)]
    pub quote_content: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct TranscriptMessageListItem {
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
    pub category: String,
    pub content: Option<String>,
    pub status: MessageStatus,
    pub created_at: DateTime<Utc>,
    pub media_url: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub media_duration: Option<String>,
    pub media_width: Option<i32>,
    pub media_height: Option<i32>,
    pub thumb_image: Option<String>,
    pub media_status: Option<MediaStatus>,
    pub quote_message_id: Option<String>,
    pub quote_content: Option<String>,
    pub quote_user_membership: Option<String>,
    pub caption: Option<String>,
    pub media_name: Option<String>,
    pub sticker_id: Option<String>,
    pub shared_user_id: Option<String>,
    pub media_waveform: Option<String>,
    pub thumb_url: Option<String>,
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
}

impl TranscriptMessageDao {
    pub async fn insert_all(&self, transcripts: &[TranscriptMessage]) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        Self::insert_all_with(&mut transaction, transcripts).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub(crate) async fn insert_all_with(
        transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        transcripts: &[TranscriptMessage],
    ) -> Result<(), Error> {
        for transcript in transcripts {
            sqlx::query(
                r#"INSERT OR REPLACE INTO transcript_messages
                   (transcript_id, message_id, user_id, user_full_name, category, created_at,
                    content, media_url, media_name, media_size, media_width, media_height,
                    media_mime_type, media_duration, media_status, media_waveform, thumb_image,
                    thumb_url, media_key, media_digest, media_created_at, sticker_id,
                    shared_user_id, mentions, quote_id, quote_content, caption)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                           ?, ?, ?, ?)"#,
            )
            .bind(&transcript.transcript_id)
            .bind(&transcript.message_id)
            .bind(&transcript.user_id)
            .bind(&transcript.user_full_name)
            .bind(&transcript.category)
            .bind(transcript.created_at)
            .bind(&transcript.content)
            .bind(&transcript.media_url)
            .bind(&transcript.media_name)
            .bind(transcript.media_size)
            .bind(transcript.media_width)
            .bind(transcript.media_height)
            .bind(&transcript.media_mime_type)
            .bind(&transcript.media_duration)
            .bind(&transcript.media_status)
            .bind(&transcript.media_waveform)
            .bind(&transcript.thumb_image)
            .bind(&transcript.thumb_url)
            .bind(&transcript.media_key)
            .bind(&transcript.media_digest)
            .bind(
                transcript
                    .media_created_at
                    .map(|created_at| created_at.timestamp_millis()),
            )
            .bind(&transcript.sticker_id)
            .bind(&transcript.shared_user_id)
            .bind(&transcript.mentions)
            .bind(&transcript.quote_id)
            .bind(&transcript.quote_content)
            .bind(&transcript.caption)
            .execute(&mut **transaction)
            .await?;
        }
        Ok(())
    }

    pub async fn update_media_status(
        &self,
        transcript_id: &str,
        message_id: &str,
        status: MediaStatus,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE transcript_messages SET media_status = ? \
             WHERE transcript_id = ? AND message_id = ?",
        )
        .bind(status)
        .bind(transcript_id)
        .bind(message_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn complete_attachment_upload(
        &self,
        transcript_id: &str,
        message_id: &str,
        content: &str,
        media_key: Option<&str>,
        media_digest: Option<&str>,
        media_created_at: DateTime<Utc>,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE transcript_messages SET content = ?, media_key = ?, media_digest = ?, \
             media_created_at = ?, media_status = ? \
             WHERE transcript_id = ? AND message_id = ?",
        )
        .bind(content)
        .bind(media_key)
        .bind(media_digest)
        .bind(media_created_at.timestamp_millis())
        .bind(MediaStatus::Done)
        .bind(transcript_id)
        .bind(message_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn complete_attachment_download(
        &self,
        transcript_id: &str,
        message_id: &str,
        media_url: &str,
        media_size: i64,
        media_created_at: Option<DateTime<Utc>>,
        content: &str,
    ) -> Result<(), Error> {
        sqlx::query(
            "UPDATE transcript_messages SET media_url = ?, media_size = ?, media_status = ?, \
             media_created_at = ?, content = ? WHERE transcript_id = ? AND message_id = ?",
        )
        .bind(media_url)
        .bind(media_size)
        .bind(MediaStatus::Done)
        .bind(media_created_at.map(|created_at| created_at.timestamp_millis()))
        .bind(content)
        .bind(transcript_id)
        .bind(message_id)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn complete_attachment_download_if_pending(
        &self,
        transcript_id: &str,
        message_id: &str,
        media_url: &str,
        media_size: i64,
        media_created_at: Option<DateTime<Utc>>,
        content: &str,
    ) -> Result<bool, Error> {
        let result = sqlx::query(
            "UPDATE transcript_messages SET media_url = ?, media_size = ?, media_status = ?, \
             media_created_at = ?, content = ? WHERE transcript_id = ? AND message_id = ? \
             AND media_status = ?",
        )
        .bind(media_url)
        .bind(media_size)
        .bind(MediaStatus::Done)
        .bind(media_created_at.map(|created_at| created_at.timestamp_millis()))
        .bind(content)
        .bind(transcript_id)
        .bind(message_id)
        .bind(MediaStatus::Pending)
        .execute(&self.0)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn find_by_transcript_id(
        &self,
        transcript_id: &str,
    ) -> Result<Vec<TranscriptMessage>, Error> {
        let query = format!(
            "SELECT {TRANSCRIPT_SELECT_COLUMNS} FROM transcript_messages \
             WHERE transcript_id = ? ORDER BY created_at"
        );
        Ok(
            sqlx::query_as::<_, TranscriptMessage>(sqlx::AssertSqlSafe(query))
                .bind(transcript_id)
                .fetch_all(&self.0)
                .await?,
        )
    }

    pub async fn list_items(
        &self,
        transcript_id: &str,
    ) -> Result<Vec<TranscriptMessageListItem>, Error> {
        Ok(sqlx::query_as::<_, TranscriptMessageListItem>(
            r#"SELECT transcript.message_id AS message_id,
                       parent.conversation_id AS conversation_id,
                       COALESCE(transcript.user_id, '') AS user_id,
                       COALESCE(sender.full_name, transcript.user_full_name, '') AS sender_name,
                       COALESCE(sender.identity_number, '') AS sender_identity_number,
                       COALESCE(sender.avatar_url, '') AS sender_avatar_url,
                       COALESCE(sender.is_verified, FALSE) AS sender_is_verified,
                       sender.membership AS sender_membership,
                       COALESCE(sender.relationship, '') AS sender_relationship,
                       sender.app_id AS sender_app_id,
                       COALESCE(sender.is_scam, FALSE) AS sender_is_scam,
                       (sender.app_id IS NOT NULL AND sender.app_id != '') AS sender_is_bot,
                       transcript.category AS category,
                       transcript.content AS content,
                       parent.status AS status,
                       CASE WHEN typeof(transcript.created_at) = 'integer'
                            THEN strftime('%Y-%m-%dT%H:%M:%fZ', transcript.created_at / 1000.0, 'unixepoch')
                            ELSE transcript.created_at END AS created_at,
                       transcript.media_url AS media_url,
                       transcript.media_mime_type AS media_mime_type,
                       transcript.media_size AS media_size,
                       transcript.media_duration AS media_duration,
                       transcript.media_width AS media_width,
                       transcript.media_height AS media_height,
                       transcript.thumb_image AS thumb_image,
                       transcript.media_status AS media_status,
                       transcript.quote_id AS quote_message_id,
                       transcript.quote_content AS quote_content,
                       quote_user.membership AS quote_user_membership,
                       transcript.caption AS caption,
                       transcript.media_name AS media_name,
                       transcript.sticker_id AS sticker_id,
                       transcript.shared_user_id AS shared_user_id,
                       transcript.media_waveform AS media_waveform,
                       transcript.thumb_url AS thumb_url,
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
                       sticker.asset_type AS sticker_asset_type
                FROM transcript_messages AS transcript
                INNER JOIN messages AS parent ON parent.message_id = transcript.transcript_id
                LEFT JOIN users AS sender ON sender.user_id = transcript.user_id
                LEFT JOIN users AS shared_user ON shared_user.user_id = transcript.shared_user_id
                LEFT JOIN users AS quote_user ON quote_user.user_id = CASE WHEN json_valid(transcript.quote_content) THEN json_extract(transcript.quote_content, '$.user_id') END
                LEFT JOIN stickers AS sticker ON sticker.sticker_id = transcript.sticker_id
                WHERE transcript.transcript_id = ?
                ORDER BY transcript.created_at, transcript.rowid"#,
        )
        .bind(transcript_id)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn find(
        &self,
        transcript_id: &str,
        message_id: &str,
    ) -> Result<Option<TranscriptMessage>, Error> {
        let query = format!(
            "SELECT {TRANSCRIPT_SELECT_COLUMNS} FROM transcript_messages \
             WHERE transcript_id = ? AND message_id = ? LIMIT 1"
        );
        Ok(
            sqlx::query_as::<_, TranscriptMessage>(sqlx::AssertSqlSafe(query))
                .bind(transcript_id)
                .bind(message_id)
                .fetch_optional(&self.0)
                .await?,
        )
    }

    pub async fn media_urls_by_transcript_id(
        &self,
        transcript_id: &str,
    ) -> Result<Vec<String>, Error> {
        Ok(sqlx::query_scalar(
            "SELECT media_url FROM transcript_messages \
             WHERE transcript_id = ? AND media_url IS NOT NULL AND media_url != ''",
        )
        .bind(transcript_id)
        .fetch_all(&self.0)
        .await?)
    }
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_datetime_value(serde_json::Value::deserialize(deserializer)?)
        .ok_or_else(|| D::Error::custom("invalid transcript datetime"))
}

fn deserialize_optional_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if value.is_null() {
        return Ok(None);
    }
    deserialize_datetime_value(value)
        .map(Some)
        .ok_or_else(|| D::Error::custom("invalid optional transcript datetime"))
}

fn deserialize_datetime_value(value: serde_json::Value) -> Option<DateTime<Utc>> {
    match value {
        serde_json::Value::String(value) => DateTime::parse_from_rfc3339(&value)
            .ok()
            .map(|value| value.with_timezone(&Utc)),
        serde_json::Value::Number(value) => value
            .as_i64()
            .and_then(DateTime::<Utc>::from_timestamp_millis),
        _ => None,
    }
}

fn serialize_datetime<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_rfc3339())
}

fn serialize_optional_datetime<S>(
    value: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(&value.to_rfc3339()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(value),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => {
            return Err(D::Error::custom(
                "transcript value is not a string or number",
            ))
        }
    })
}

fn serialize_optional_number<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => match value.parse::<i64>() {
            Ok(value) => serializer.serialize_some(&value),
            Err(_) => serializer.serialize_none(),
        },
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MixinDatabase;

    #[tokio::test]
    async fn parses_and_persists_flutter_transcript_payload() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let mut transcripts: Vec<TranscriptMessage> = serde_json::from_str(
            r#"[{
                "transcript_id":"root","message_id":"child","user_id":"sender",
                "user_full_name":"Mixin","category":"SIGNAL_AUDIO",
                "created_at":"2024-01-02T03:04:05Z","content":"attachment-id",
                "media_size":42,"media_duration":12,"media_status":"DONE",
                "media_created_at":"2024-01-02T03:04:06.123Z"
            }]"#,
        )
        .unwrap();
        transcripts[0].media_status = Some(MediaStatus::Canceled);

        database
            .transcript_message_dao
            .insert_all(&transcripts)
            .await
            .unwrap();
        let stored = database
            .transcript_message_dao
            .find_by_transcript_id("root")
            .await
            .unwrap();

        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].media_duration.as_deref(), Some("12"));
        assert_eq!(stored[0].media_status, Some(MediaStatus::Canceled));
        assert_eq!(
            stored[0].media_created_at.unwrap().timestamp_millis(),
            1_704_164_646_123
        );
        let storage_type: String = sqlx::query_scalar(
            "SELECT typeof(media_created_at) FROM transcript_messages \
             WHERE transcript_id = 'root' AND message_id = 'child'",
        )
        .fetch_one(&database.transcript_message_dao.0)
        .await
        .unwrap();
        assert_eq!(storage_type, "integer");
        let wire = serde_json::to_value(&stored[0]).unwrap();
        assert_eq!(wire["created_at"], "2024-01-02T03:04:05+00:00");
        assert_eq!(wire["media_duration"], 12);
        assert!(wire.get("media_status").is_none());

        let completed_at = DateTime::parse_from_rfc3339("2024-01-02T03:05:00.456Z")
            .unwrap()
            .with_timezone(&Utc);
        database
            .transcript_message_dao
            .complete_attachment_download(
                "root",
                "child",
                "/Media/Transcripts/child.ogg",
                64,
                Some(completed_at),
                r#"{"attachment_id":"fresh-id","message_id":"child"}"#,
            )
            .await
            .unwrap();
        let completed = database
            .transcript_message_dao
            .find_by_transcript_id("root")
            .await
            .unwrap();
        assert_eq!(completed[0].media_status, Some(MediaStatus::Done));
        assert_eq!(completed[0].media_size, Some(64));
        assert_eq!(completed[0].media_created_at, Some(completed_at));
        assert_eq!(
            completed[0].content.as_deref(),
            Some(r#"{"attachment_id":"fresh-id","message_id":"child"}"#)
        );
    }
}
