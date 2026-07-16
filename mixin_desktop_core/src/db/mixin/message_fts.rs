use sqlx::{FromRow, Sqlite};

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
        let mut transaction = self.0.begin().await?;
        sqlx::query("DELETE FROM message_fts WHERE message_id = ?")
            .bind(message_id)
            .execute(&mut *transaction)
            .await?;

        let content = normalize_content(content);
        if !content.trim().is_empty() {
            sqlx::query(
                "INSERT INTO message_fts (message_id, conversation_id, content) VALUES (?, ?, ?)",
            )
            .bind(message_id)
            .bind(conversation_id)
            .bind(content)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn delete_by_message_id(&self, message_id: &str) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM message_fts WHERE message_id = ?")
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_by_conversation_id(&self, conversation_id: &str) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM message_fts WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
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
                "SELECT message_id, conversation_id, content FROM message_fts \
                 WHERE message_fts MATCH ? AND conversation_id = ? \
                 ORDER BY rank LIMIT ? OFFSET ?",
            )
            .bind(query)
            .bind(conversation_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.0)
            .await?
        } else {
            sqlx::query_as::<_, MessageFtsItem>(
                "SELECT message_id, conversation_id, content FROM message_fts \
                 WHERE message_fts MATCH ? ORDER BY rank LIMIT ? OFFSET ?",
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
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::MessageFtsDao;
    use crate::db::mixin::MixinDatabase;

    #[tokio::test]
    async fn upserts_searches_and_deletes_fts_messages() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = database.message_fts_dao;

        dao.upsert("one", "conversation-one", "hello world")
            .await
            .unwrap();
        dao.upsert("two", "conversation-two", "hello mixin")
            .await
            .unwrap();
        assert_eq!(dao.search("hel", None, 10).await.unwrap().len(), 2);
        assert_eq!(dao.search_range("hel", None, 1, 1).await.unwrap().len(), 1);
        assert_eq!(
            dao.search("hello", Some("conversation-one"), 10)
                .await
                .unwrap()
                .iter()
                .map(|item| item.message_id.as_str())
                .collect::<Vec<_>>(),
            vec!["one"]
        );

        dao.upsert("one", "conversation-one", "replacement")
            .await
            .unwrap();
        assert!(dao.search("world", None, 10).await.unwrap().is_empty());
        assert_eq!(
            dao.search("replace", None, 10).await.unwrap()[0].message_id,
            "one"
        );

        assert_eq!(dao.delete_by_message_id("one").await.unwrap(), 1);
        assert!(dao.search("replace", None, 10).await.unwrap().is_empty());
        assert_eq!(
            dao.delete_by_conversation_id("conversation-two")
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn escapes_quotes_and_indexes_non_ascii_characters() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = database.message_fts_dao;

        dao.upsert("quoted", "conversation", "say hello")
            .await
            .unwrap();
        dao.upsert("chinese", "conversation", "你好 Mixin")
            .await
            .unwrap();

        assert_eq!(dao.search("\"hello\"", None, 10).await.unwrap().len(), 1);
        assert_eq!(dao.search("你", None, 10).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn migration_backfills_flutter_searchable_content() {
        let directory = tempfile::tempdir().unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(directory.path().join("mixin.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("migrations/01_init.up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO users (user_id, identity_number, full_name) VALUES ('shared', '1', 'Alice')",
        )
        .execute(&pool)
        .await
        .unwrap();
        for (message_id, category, content, name, shared_user_id) in [
            ("text", "PLAIN_TEXT", Some("你好"), None, None),
            ("data", "PLAIN_DATA", None, Some("report.pdf"), None),
            ("contact", "PLAIN_CONTACT", None, None, Some("shared")),
            (
                "card",
                "APP_CARD",
                Some(r#"{"title":"Card title","description":"Card body"}"#),
                None,
                None,
            ),
            ("transcript", "PLAIN_TRANSCRIPT", Some("[]"), None, None),
        ] {
            sqlx::query(
                "INSERT INTO messages \
                 (message_id, conversation_id, user_id, category, content, name, shared_user_id, status, created_at) \
                 VALUES (?, 'conversation', 'sender', ?, ?, ?, ?, 'SENT', '2024-01-01 00:00:00')",
            )
            .bind(message_id)
            .bind(category)
            .bind(content)
            .bind(name)
            .bind(shared_user_id)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO transcript_messages \
             (transcript_id, message_id, category, created_at, content) \
             VALUES ('transcript', 'child', 'PLAIN_TEXT', '2024-01-01 00:00:00', 'child words')",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::raw_sql(include_str!("migrations/03_message_fts.up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        let dao = MessageFtsDao(pool);

        for (query, message_id) in [
            ("好", "text"),
            ("report", "data"),
            ("Alice", "contact"),
            ("Card", "card"),
            ("child", "transcript"),
        ] {
            assert_eq!(
                dao.search(query, None, 10).await.unwrap()[0].message_id,
                message_id
            );
        }
    }
}
