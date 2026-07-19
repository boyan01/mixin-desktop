use sqlx::{FromRow, Row, Sqlite};

use sdk::blaze_message::MessageStatus;
use sdk::message_category::MessageCategory;

use crate::db::mixin::message::Message;
use crate::db::Error;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct MessageFtsItem {
    pub message_id: String,
    pub conversation_id: String,
    pub content: String,
}

#[derive(Clone)]
pub struct MessageFtsDao(pub(crate) sqlx::Pool<Sqlite>);

impl MessageFtsDao {
    pub async fn upsert(
        &self,
        message_id: &str,
        conversation_id: &str,
        content: &str,
    ) -> Result<(), Error> {
        let metadata = sqlx::query(
            r#"SELECT category, user_id,
               CASE WHEN typeof(created_at) = 'integer' THEN created_at
                    ELSE CAST(strftime('%s', created_at) AS INTEGER) * 1000 END AS created_at
               FROM messages WHERE message_id = ? AND conversation_id = ?"#,
        )
        .bind(message_id)
        .bind(conversation_id)
        .fetch_optional(&self.0)
        .await?;
        let Some(metadata) = metadata else {
            return Ok(());
        };

        let mut transaction = self.0.begin().await?;
        delete_message_fts(&mut transaction, message_id).await?;

        let content = normalize_content(content);
        if !content.trim().is_empty() {
            let doc_id = sqlx::query("INSERT INTO fts.messages_fts (content) VALUES (?)")
                .bind(content)
                .execute(&mut *transaction)
                .await?
                .last_insert_rowid();
            sqlx::query(
                "INSERT INTO fts.messages_metas \
                 (doc_id, message_id, conversation_id, category, user_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(doc_id)
            .bind(message_id)
            .bind(conversation_id)
            .bind(metadata.get::<String, _>("category"))
            .bind(metadata.get::<String, _>("user_id"))
            .bind(metadata.get::<i64, _>("created_at"))
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn delete_by_message_id(&self, message_id: &str) -> Result<u64, Error> {
        let mut transaction = self.0.begin().await?;
        let deleted = delete_message_fts(&mut transaction, message_id).await?;
        transaction.commit().await?;
        Ok(deleted)
    }

    pub async fn delete_by_conversation_id(&self, conversation_id: &str) -> Result<u64, Error> {
        let mut transaction = self.0.begin().await?;
        let deleted = delete_conversation_fts(&mut transaction, conversation_id).await?;
        transaction.commit().await?;
        Ok(deleted)
    }

    pub(crate) async fn migrate_batch(&self, anchor: Option<i64>) -> Result<Option<i64>, Error> {
        let mut transaction = self.0.begin().await?;
        let rows = sqlx::query(
            r#"SELECT m.rowid AS source_rowid, m.message_id, m.conversation_id, m.category, m.user_id,
               CASE WHEN typeof(m.created_at) = 'integer' THEN m.created_at
                    ELSE CAST(strftime('%s', m.created_at) AS INTEGER) * 1000 END AS created_at,
               CASE
                 WHEN m.category LIKE '%_TEXT' OR m.category LIKE '%_POST' THEN m.content
                 WHEN m.category LIKE '%_DATA' THEN m.name
                 WHEN m.category LIKE '%_CONTACT' THEN shared_user.full_name
                 WHEN m.category = 'APP_CARD' AND json_valid(m.content) THEN
                   trim(coalesce(json_extract(m.content, '$.title'), '') || ' ' ||
                        coalesce(json_extract(m.content, '$.description'), ''))
                 WHEN m.category LIKE '%_TRANSCRIPT' THEN (
                   SELECT group_concat(
                     CASE
                       WHEN tm.category LIKE '%_TEXT' OR tm.category LIKE '%_POST' THEN tm.content
                       WHEN tm.category LIKE '%_DATA' THEN tm.media_name
                       WHEN tm.category LIKE '%_CONTACT' THEN transcript_user.full_name
                       ELSE NULL
                     END, ' ')
                   FROM transcript_messages AS tm
                   LEFT JOIN users AS transcript_user ON transcript_user.user_id = tm.shared_user_id
                   WHERE tm.transcript_id = m.message_id
                 )
                 ELSE NULL
               END AS fts_content
               FROM messages AS m
               LEFT JOIN users AS shared_user ON shared_user.user_id = m.shared_user_id
               WHERE m.rowid > ? AND m.status NOT IN ('UNKNOWN', 'FAILED')
               ORDER BY m.rowid LIMIT 1000"#,
        )
        .bind(anchor.unwrap_or(0))
        .fetch_all(&mut *transaction)
        .await?;
        let next_anchor = rows.last().map(|row| row.get::<i64, _>("source_rowid"));
        for row in rows {
            let message_id = row.get::<String, _>("message_id");
            if sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM fts.messages_metas WHERE message_id = ?)",
            )
            .bind(&message_id)
            .fetch_one(&mut *transaction)
            .await?
            {
                continue;
            }
            let Some(content) = row.get::<Option<String>, _>("fts_content") else {
                continue;
            };
            let content = normalize_content(&content);
            if content.trim().is_empty() {
                continue;
            }
            let doc_id = sqlx::query("INSERT INTO fts.messages_fts (content) VALUES (?)")
                .bind(content)
                .execute(&mut *transaction)
                .await?
                .last_insert_rowid();
            sqlx::query(
                "INSERT INTO fts.messages_metas \
                 (doc_id, message_id, conversation_id, category, user_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(doc_id)
            .bind(message_id)
            .bind(row.get::<String, _>("conversation_id"))
            .bind(row.get::<String, _>("category"))
            .bind(row.get::<String, _>("user_id"))
            .bind(row.get::<i64, _>("created_at"))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(next_anchor)
    }

    pub async fn search(
        &self,
        query: &str,
        conversation_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<MessageFtsItem>, Error> {
        self.search_range(query, conversation_id, limit, 0).await
    }

    pub async fn search_range(
        &self,
        query: &str,
        conversation_id: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageFtsItem>, Error> {
        let Some(query) = match_query(query) else {
            return Ok(Vec::new());
        };
        if limit == 0 {
            return Ok(Vec::new());
        }

        let items = if let Some(conversation_id) = conversation_id {
            sqlx::query_as::<_, MessageFtsItem>(
                "SELECT meta.message_id, meta.conversation_id, messages_fts.content \
                 FROM fts.messages_fts \
                 JOIN fts.messages_metas AS meta ON meta.doc_id = messages_fts.rowid \
                 WHERE messages_fts MATCH ? AND meta.conversation_id = ? \
                 ORDER BY meta.created_at DESC, messages_fts.rowid DESC LIMIT ? OFFSET ?",
            )
            .bind(query)
            .bind(conversation_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.0)
            .await?
        } else {
            sqlx::query_as::<_, MessageFtsItem>(
                "SELECT meta.message_id, meta.conversation_id, messages_fts.content \
                 FROM fts.messages_fts \
                 JOIN fts.messages_metas AS meta ON meta.doc_id = messages_fts.rowid \
                 WHERE messages_fts MATCH ? \
                 ORDER BY meta.created_at DESC, messages_fts.rowid DESC LIMIT ? OFFSET ?",
            )
            .bind(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.0)
            .await?
        };
        Ok(items)
    }
}

pub(crate) fn message_fts_content(message: &Message) -> Option<String> {
    if matches!(
        message.status,
        MessageStatus::Unknown | MessageStatus::Failed
    ) {
        return None;
    }
    if message.category.is_text() || message.category.is_post() {
        return message.content.clone();
    }
    if message.category.is_data() || message.category.is_contact() {
        return message.name.clone();
    }
    if message.category.is_app_card() {
        let card = serde_json::from_str::<sdk::AppCard>(message.content.as_deref()?).ok()?;
        return Some(format!("{} {}", card.title, card.description));
    }
    None
}

pub(crate) async fn delete_message_fts(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    message_id: &str,
) -> Result<u64, Error> {
    sqlx::query(
        "DELETE FROM fts.messages_fts WHERE rowid = \
         (SELECT doc_id FROM fts.messages_metas WHERE message_id = ?)",
    )
    .bind(message_id)
    .execute(&mut **transaction)
    .await?;
    Ok(
        sqlx::query("DELETE FROM fts.messages_metas WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut **transaction)
            .await?
            .rows_affected(),
    )
}

pub(crate) async fn delete_conversation_fts(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    conversation_id: &str,
) -> Result<u64, Error> {
    sqlx::query(
        "DELETE FROM fts.messages_fts WHERE rowid IN \
         (SELECT doc_id FROM fts.messages_metas WHERE conversation_id = ?)",
    )
    .bind(conversation_id)
    .execute(&mut **transaction)
    .await?;
    Ok(
        sqlx::query("DELETE FROM fts.messages_metas WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut **transaction)
            .await?
            .rows_affected(),
    )
}

fn normalize_content(content: &str) -> String {
    let mut normalized = String::with_capacity(content.len());
    let mut previous_was_ascii_alphanumeric = false;
    for character in content.trim().chars() {
        let is_ascii_alphanumeric = character.is_ascii_alphanumeric();
        if previous_was_ascii_alphanumeric && !is_ascii_alphanumeric {
            normalized.push(' ');
        }
        normalized.push(character);
        if !is_ascii_alphanumeric {
            normalized.push(' ');
        }
        previous_was_ascii_alphanumeric = is_ascii_alphanumeric;
    }
    normalized
}

fn match_query(query: &str) -> Option<String> {
    let normalized = normalize_content(&query.replace('"', ""));
    let tokens = normalized
        .split_whitespace()
        .map(|token| format!("\"{token}\"*"))
        .collect::<Vec<_>>();
    (!tokens.is_empty()).then(|| tokens.join(" "))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::db::mixin::message::Message;
    use crate::db::mixin::MixinDatabase;

    #[tokio::test]
    async fn uses_flutter_fts_database() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('conversation', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        database
            .message_dao
            .insert_message(&Message {
                message_id: "message".into(),
                conversation_id: "conversation".into(),
                user_id: "user".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("hello mixin".into()),
                status: sdk::blaze_message::MessageStatus::Sent,
                created_at: Utc::now().naive_utc(),
                ..Message::default()
            })
            .await
            .unwrap();

        database
            .message_fts_dao
            .upsert("message", "conversation", "hello mixin")
            .await
            .unwrap();
        assert_eq!(
            database
                .message_fts_dao
                .search("hello", None, 10)
                .await
                .unwrap()[0]
                .message_id,
            "message"
        );
        let version: i64 = sqlx::query_scalar("PRAGMA fts.user_version")
            .fetch_one(&database.message_fts_dao.0)
            .await
            .unwrap();
        assert_eq!(version, 1);
    }
}
