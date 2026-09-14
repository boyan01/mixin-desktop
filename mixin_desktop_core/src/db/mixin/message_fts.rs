use sqlx::{FromRow, QueryBuilder, Row, Sqlite};

use sdk::blaze_message::MessageStatus;
use sdk::message_category::MessageCategory;

use crate::db::mixin::message::Message;
use crate::db::Error;

#[derive(Debug, Clone, PartialEq, Eq, FromRow)]
pub struct MessageFtsItem {
    pub message_id: String,
}

#[derive(Clone)]
pub struct MessageFtsDao(pub(crate) sqlx::Pool<Sqlite>);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageFtsSearch {
    pub query: String,
    pub conversation_ids: Vec<String>,
    pub sender_ids: Vec<String>,
    pub categories: Vec<String>,
    pub start_created_at_millis: Option<i64>,
    pub end_created_at_millis: Option<i64>,
    pub cursor_message_id: Option<String>,
    pub limit: u32,
}

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

        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
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
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let deleted = delete_message_fts(&mut transaction, message_id).await?;
        transaction.commit().await?;
        Ok(deleted)
    }

    pub async fn delete_by_conversation_id(&self, conversation_id: &str) -> Result<u64, Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let deleted = delete_conversation_fts(&mut transaction, conversation_id).await?;
        transaction.commit().await?;
        Ok(deleted)
    }

    pub(crate) async fn migrate_batch(&self, anchor: Option<i64>) -> Result<Option<i64>, Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
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

    pub async fn search(&self, input: &MessageFtsSearch) -> Result<Vec<MessageFtsItem>, Error> {
        let mut connection = self.0.acquire().await?;
        Self::search_on(&mut connection, input).await
    }

    pub(crate) async fn search_bounded(
        &self,
        input: &MessageFtsSearch,
        cancellation: tokio_util::sync::CancellationToken,
        budget: std::time::Duration,
    ) -> anyhow::Result<Vec<MessageFtsItem>> {
        let cancellation = cancellation.child_token();
        let started = std::time::Instant::now();
        let deadline = started + budget;
        let cancel_on_drop = cancellation.clone().drop_guard();
        // Detach so a cancelled query can never return a poisoned progress handler to the pool.
        // The pool can replenish its normal read capacity while this search runs.
        let mut connection = tokio::select! {
            _ = cancellation.cancelled() => anyhow::bail!("search_cancelled"),
            result = tokio::time::timeout(budget, self.0.acquire()) => {
                result.map_err(|_| anyhow::anyhow!("search_timeout: connection acquisition exceeded the search deadline"))??.detach()
            }
        };
        let wait = started.elapsed();
        install_search_progress_handler(&mut connection, cancellation.clone(), deadline).await?;
        let result = Self::search_on(&mut connection, input).await;
        let elapsed = started.elapsed();
        let outcome = if cancellation.is_cancelled() {
            "cancelled"
        } else if std::time::Instant::now() >= deadline {
            "timeout"
        } else if result.is_err() {
            "error"
        } else {
            "ok"
        };
        log::info!("MCP search outcome={outcome} connection_wait_ms={} elapsed_ms={} conversation_filters={} limit={}",
            wait.as_millis(), elapsed.as_millis(), input.conversation_ids.len(), input.limit);
        drop(cancel_on_drop);
        match outcome {
            "cancelled" => anyhow::bail!("search_cancelled"),
            "timeout" => anyhow::bail!(
                "search_timeout: search exceeded {} seconds; narrow the time range before retrying",
                budget.as_secs()
            ),
            _ => Ok(result?),
        }
    }

    async fn search_on(
        connection: &mut sqlx::SqliteConnection,
        input: &MessageFtsSearch,
    ) -> Result<Vec<MessageFtsItem>, Error> {
        let Some(query) = match_query(&input.query) else {
            return Ok(Vec::new());
        };
        if input.limit == 0 {
            return Ok(Vec::new());
        }

        let anchor = if let Some(message_id) = input.cursor_message_id.as_deref() {
            let Some(anchor) = sqlx::query_as::<_, (i64, i64)>(
                "SELECT created_at, rowid FROM fts.messages_metas WHERE message_id = ? LIMIT 1",
            )
            .bind(message_id)
            .fetch_optional(&mut *connection)
            .await?
            else {
                return Ok(Vec::new());
            };
            Some(anchor)
        } else {
            None
        };

        // Bound the FTS posting-list scan only when the caller supplies a finite time window.
        // Materializing candidates prevents SQLite from expanding the global MATCH first.
        let bounded_window =
            input.start_created_at_millis.is_some() && input.end_created_at_millis.is_some();
        let mut builder = if bounded_window {
            QueryBuilder::<Sqlite>::new(
                "WITH candidates AS MATERIALIZED (SELECT meta.message_id, meta.doc_id, \
                 meta.created_at, meta.rowid AS meta_rowid FROM fts.messages_metas meta WHERE 1=1",
            )
        } else {
            let mut builder = QueryBuilder::<Sqlite>::new(
                "SELECT meta.message_id FROM fts.messages_metas meta \
                 WHERE meta.doc_id IN (SELECT rowid FROM fts.messages_fts \
                 WHERE messages_fts MATCH ",
            );
            builder.push_bind(&query).push(")");
            builder
        };
        if !input.conversation_ids.is_empty() {
            builder.push(" AND meta.conversation_id IN (");
            let mut separated = builder.separated(", ");
            for conversation_id in &input.conversation_ids {
                separated.push_bind(conversation_id);
            }
            separated.push_unseparated(")");
        }
        if !input.sender_ids.is_empty() {
            builder.push(" AND meta.user_id IN (");
            let mut separated = builder.separated(", ");
            for sender_id in &input.sender_ids {
                separated.push_bind(sender_id);
            }
            separated.push_unseparated(")");
        }
        if !input.categories.is_empty() {
            builder.push(" AND meta.category IN (");
            let mut separated = builder.separated(", ");
            for category in &input.categories {
                separated.push_bind(category);
            }
            separated.push_unseparated(")");
        }
        if let Some(start) = input.start_created_at_millis {
            builder.push(" AND meta.created_at >= ").push_bind(start);
        }
        if let Some(end) = input.end_created_at_millis {
            builder.push(" AND meta.created_at < ").push_bind(end);
        }
        if let Some((created_at, row_id)) = anchor {
            builder
                .push(" AND (meta.created_at < ")
                .push_bind(created_at)
                .push(" OR (meta.created_at = ")
                .push_bind(created_at)
                .push(" AND meta.rowid < ")
                .push_bind(row_id)
                .push("))");
        }
        if bounded_window {
            builder
                .push(
                    ") SELECT message_id FROM candidates WHERE doc_id IN (\
                 SELECT rowid FROM fts.messages_fts WHERE messages_fts MATCH ",
                )
                .push_bind(&query)
                .push(
                    " AND rowid >= (SELECT min(doc_id) FROM candidates) \
                 AND rowid <= (SELECT max(doc_id) FROM candidates)) \
                 ORDER BY created_at DESC, meta_rowid DESC LIMIT ",
                );
        } else {
            builder.push(" ORDER BY meta.created_at DESC, meta.rowid DESC LIMIT ");
        }
        builder.push_bind(input.limit.clamp(1, 200));
        Ok(builder
            .build_query_as::<MessageFtsItem>()
            .fetch_all(&mut *connection)
            .await?)
    }
}

async fn install_search_progress_handler(
    connection: &mut sqlx::SqliteConnection,
    cancellation: tokio_util::sync::CancellationToken,
    deadline: std::time::Instant,
) -> Result<(), sqlx::Error> {
    connection
        .lock_handle()
        .await?
        .set_progress_handler(1000, move || {
            !cancellation.is_cancelled() && std::time::Instant::now() < deadline
        });
    Ok(())
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

    use super::MessageFtsSearch;

    #[tokio::test]
    async fn bounded_window_avoids_expanding_historical_matches() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let mut connection = database.message_fts_dao.0.acquire().await.unwrap();
        sqlx::raw_sql("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<20000)
            INSERT INTO fts.messages_fts(rowid,content) SELECT x,'needle' FROM n;
            INSERT INTO fts.messages_metas(doc_id,message_id,conversation_id,category,user_id,created_at)
            SELECT rowid,CAST(rowid AS TEXT),'scope','PLAIN_TEXT','alice',rowid FROM fts.messages_fts;")
            .execute(&mut *connection).await.unwrap();
        let steps = Arc::new(AtomicUsize::new(0));
        let counter = steps.clone();
        connection
            .lock_handle()
            .await
            .unwrap()
            .set_progress_handler(100, move || {
                counter.fetch_add(1, Ordering::Relaxed);
                true
            });
        let input = MessageFtsSearch {
            query: "needle".into(),
            conversation_ids: vec!["scope".into()],
            start_created_at_millis: Some(19990),
            end_created_at_millis: Some(20001),
            limit: 5,
            ..Default::default()
        };
        let optimized = super::MessageFtsDao::search_on(&mut connection, &input)
            .await
            .unwrap();
        let optimized_steps = steps.swap(0, Ordering::Relaxed);
        let original = sqlx::query_as::<_, super::MessageFtsItem>(
            "SELECT meta.message_id FROM fts.messages_metas meta WHERE meta.doc_id IN
             (SELECT rowid FROM fts.messages_fts WHERE messages_fts MATCH ?)
             AND meta.conversation_id IN (?) AND meta.created_at >= ? AND meta.created_at < ?
             ORDER BY meta.created_at DESC, meta.rowid DESC LIMIT ?",
        )
        .bind("\"needle\"*")
        .bind("scope")
        .bind(19990_i64)
        .bind(20001_i64)
        .bind(5_i64)
        .fetch_all(&mut *connection)
        .await
        .unwrap();
        let original_steps = steps.load(Ordering::Relaxed);
        connection
            .lock_handle()
            .await
            .unwrap()
            .remove_progress_handler();
        assert_eq!(optimized, original);
        assert!(
            optimized_steps * 2 < original_steps,
            "bounded={optimized_steps}, original={original_steps}"
        );
        drop(connection);
        database.close().await;
    }

    #[tokio::test]
    async fn bounded_window_preserves_filters_cursor_ties_and_nonchronological_doc_ids() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        for (doc_id, id, conversation, user, category, timestamp, content) in [
            (100, "old", "a", "alice", "PLAIN_TEXT", 100, "needle"),
            (3, "tie1", "b", "alice", "PLAIN_TEXT", 150, "needle"),
            (50, "tie2", "a", "alice", "PLAIN_TEXT", 150, "needles"),
            (40, "wrong-user", "a", "bob", "PLAIN_TEXT", 160, "needle"),
            (
                45,
                "wrong-category",
                "a",
                "alice",
                "PLAIN_IMAGE",
                170,
                "needle",
            ),
            (
                46,
                "wrong-conversation",
                "c",
                "alice",
                "PLAIN_TEXT",
                180,
                "needle",
            ),
            (
                47,
                "wrong-text",
                "a",
                "alice",
                "PLAIN_TEXT",
                180,
                "different",
            ),
            (48, "end", "a", "alice", "PLAIN_TEXT", 200, "needle"),
        ] {
            sqlx::query("INSERT INTO fts.messages_fts(rowid,content) VALUES (?,?)")
                .bind(doc_id)
                .bind(content)
                .execute(&database.message_fts_dao.0)
                .await
                .unwrap();
            sqlx::query("INSERT INTO fts.messages_metas(doc_id,message_id,conversation_id,user_id,category,created_at) VALUES (?,?,?,?,?,?)")
                .bind(doc_id).bind(id).bind(conversation).bind(user).bind(category).bind(timestamp)
                .execute(&database.message_fts_dao.0).await.unwrap();
        }
        let mut input = MessageFtsSearch {
            query: "needle".into(),
            conversation_ids: vec!["a".into(), "b".into()],
            sender_ids: vec!["alice".into()],
            categories: vec!["PLAIN_TEXT".into()],
            start_created_at_millis: Some(100),
            end_created_at_millis: Some(200),
            limit: 1,
            ..Default::default()
        };
        for expected in ["tie2", "tie1", "old"] {
            let page = database.message_fts_dao.search(&input).await.unwrap();
            assert_eq!(
                page.iter()
                    .map(|item| item.message_id.as_str())
                    .collect::<Vec<_>>(),
                [expected]
            );
            input.cursor_message_id = Some(expected.into());
        }
        assert!(database
            .message_fts_dao
            .search(&input)
            .await
            .unwrap()
            .is_empty());
        input.cursor_message_id = None;
        input.conversation_ids = vec!["missing".into()];
        assert!(database
            .message_fts_dao
            .search(&input)
            .await
            .unwrap()
            .is_empty());
        database.close().await;
    }

    #[tokio::test]
    async fn bounded_search_times_out_expensive_sql_and_leaves_pool_available() {
        use std::time::Duration;
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql("ATTACH DATABASE ':memory:' AS fts;
            CREATE VIRTUAL TABLE fts.messages_fts USING fts5(content);
            INSERT INTO fts.messages_fts(content) VALUES ('needle');
            CREATE VIEW fts.messages_metas AS
                WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<1000000000)
                SELECT 1 AS doc_id, CAST(x AS TEXT) AS message_id, x AS created_at, x AS rowid FROM n;")
            .execute(&pool).await.unwrap();
        let dao = super::MessageFtsDao(pool.clone());
        let error = tokio::time::timeout(
            Duration::from_secs(2),
            dao.search_bounded(
                &MessageFtsSearch {
                    query: "needle".into(),
                    limit: 1,
                    ..Default::default()
                },
                tokio_util::sync::CancellationToken::new(),
                Duration::from_millis(10),
            ),
        )
        .await
        .expect("bounded search must stop SQLite")
        .unwrap_err();
        assert!(error.to_string().starts_with("search_timeout:"), "{error}");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT 42")
                .fetch_one(&pool)
                .await
                .unwrap(),
            42
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn search_progress_interrupts_expired_and_cancelled_sql_without_blocking_reads() {
        use std::time::{Duration, Instant};
        use tokio_util::sync::CancellationToken;
        for cancelled in [false, true] {
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            let mut search = pool.acquire().await.unwrap().detach();
            let cancellation = CancellationToken::new();
            super::install_search_progress_handler(
                &mut search,
                cancellation.clone(),
                Instant::now() + Duration::from_millis(10),
            )
            .await
            .unwrap();
            if cancelled {
                cancellation.cancel();
            }
            let error = tokio::time::timeout(Duration::from_secs(2),
                sqlx::query("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<1000000000) SELECT sum(x) FROM n")
                    .fetch_one(&mut search)).await.expect("SQLite must stop executing").unwrap_err();
            assert!(error.to_string().contains("interrupted"), "{error}");
            // A detached search must not exhaust even a one-connection read pool.
            assert_eq!(
                tokio::time::timeout(
                    Duration::from_secs(2),
                    sqlx::query_scalar::<_, i64>("SELECT 42").fetch_one(&pool)
                )
                .await
                .unwrap()
                .unwrap(),
                42
            );
            drop(search);
            pool.close().await;
        }
    }

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
        let now = Utc::now().naive_utc();
        for (message_id, user_id, category, offset) in [
            ("old", "alice", "PLAIN_TEXT", 0),
            ("middle", "bob", "PLAIN_IMAGE", 1),
            ("new", "alice", "PLAIN_TEXT", 2),
        ] {
            database
                .message_dao
                .insert_message(&Message {
                    message_id: message_id.into(),
                    conversation_id: "conversation".into(),
                    user_id: user_id.into(),
                    category: category.into(),
                    content: Some("hello mixin".into()),
                    status: sdk::blaze_message::MessageStatus::Sent,
                    created_at: now + chrono::Duration::seconds(offset),
                    ..Message::default()
                })
                .await
                .unwrap();
            database
                .message_fts_dao
                .upsert(message_id, "conversation", "hello mixin")
                .await
                .unwrap();
        }
        assert_eq!(
            database
                .message_fts_dao
                .search(&MessageFtsSearch {
                    query: "hello".into(),
                    limit: 2,
                    ..Default::default()
                })
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>(),
            ["new", "middle"]
        );
        assert_eq!(
            database
                .message_fts_dao
                .search(&MessageFtsSearch {
                    query: "hello".into(),
                    conversation_ids: vec!["conversation".into()],
                    sender_ids: vec!["alice".into()],
                    categories: vec!["PLAIN_TEXT".into()],
                    cursor_message_id: Some("new".into()),
                    limit: 10,
                    ..Default::default()
                })
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>(),
            ["old"]
        );
        sqlx::query(
            "INSERT INTO conversations (conversation_id, created_at, status) \
             VALUES ('other-conversation', 0, 0)",
        )
        .execute(&database.message_dao.0)
        .await
        .unwrap();
        let other_created_at = now + chrono::Duration::seconds(3);
        database
            .message_dao
            .insert_message(&Message {
                message_id: "other".into(),
                conversation_id: "other-conversation".into(),
                user_id: "bob".into(),
                category: "PLAIN_TEXT".into(),
                content: Some("hello mixin".into()),
                status: sdk::blaze_message::MessageStatus::Sent,
                created_at: other_created_at,
                ..Message::default()
            })
            .await
            .unwrap();
        database
            .message_fts_dao
            .upsert("other", "other-conversation", "hello mixin")
            .await
            .unwrap();
        assert_eq!(
            database
                .message_fts_dao
                .search(&MessageFtsSearch {
                    query: "hello".into(),
                    conversation_ids: vec!["conversation".into(), "other-conversation".into()],
                    sender_ids: vec!["bob".into()],
                    start_created_at_millis: Some(other_created_at.and_utc().timestamp_millis()),
                    end_created_at_millis: Some(other_created_at.and_utc().timestamp_millis() + 1,),
                    limit: 10,
                    ..Default::default()
                })
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.message_id)
                .collect::<Vec<_>>(),
            ["other"]
        );
        let version: i64 = sqlx::query_scalar("PRAGMA fts.user_version")
            .fetch_one(&database.message_fts_dao.0)
            .await
            .unwrap();
        assert_eq!(version, 1);
    }
}
