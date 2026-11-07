use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::message::QuoteMessage;
use crate::db::mixin::util::{expand_var, BindListForQuery};
use crate::db::Error;

#[derive(Clone)]
pub struct MessageMentionDao(pub(crate) crate::db::Pool);

#[derive(Debug, PartialEq, Eq)]
pub struct MessageMention {
    pub message_id: String,
    pub conversation_id: String,
    pub has_read: bool,
}

fn parse_mention_data(
    content: Option<&str>,
    sender_id: &str,
    quote_message: Option<&QuoteMessage>,
    current_user_id: &str,
    current_user_identity_number: &str,
) -> bool {
    if let Some(quote) = quote_message {
        if quote.user_id == current_user_id && sender_id != current_user_id {
            return true;
        }
    }
    if let Some(content) = content {
        if sender_id != current_user_id
            && content.match_indices('@').any(|(index, _)| {
                content[index + 1..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    == current_user_identity_number
            })
        {
            return true;
        }
    }
    false
}

impl MessageMentionDao {
    /// Read state and its durable acknowledgements commit together, within one conversation.
    pub async fn mark_read_and_enqueue(
        &self,
        conversation_id: &str,
        message_id: Option<&str>,
    ) -> Result<u64, Error> {
        use crate::db::mixin::{job::Job, message::MessageDao};
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        let mut cursor = None::<i64>;
        let mut changed = 0;
        loop {
            let rows = crate::db::query_as::<(i64, String)>(
                "SELECT mention.rowid, mention.message_id FROM message_mentions mention \
                 INNER JOIN messages message ON message.message_id = mention.message_id \
                 AND message.conversation_id = mention.conversation_id \
                 WHERE mention.conversation_id = ? AND COALESCE(mention.has_read, FALSE) = FALSE \
                 AND (? IS NULL OR mention.message_id = ?) \
                 AND (? IS NULL OR mention.rowid > ?) ORDER BY mention.rowid ASC LIMIT 200",
            )
            .bind(conversation_id)
            .bind(message_id)
            .bind(message_id)
            .bind(cursor)
            .bind(cursor)
            .fetch_all(&mut *transaction)
            .await?;
            if rows.is_empty() {
                break;
            }
            for (row_id, id) in rows {
                let result = crate::db::query("UPDATE message_mentions SET has_read = TRUE WHERE message_id = ? AND conversation_id = ?")
                    .bind(&id).bind(conversation_id).execute(&mut *transaction).await?;
                let job = Job::create_mention_read_ack_job(conversation_id, &id);
                MessageDao::insert_job_with(&mut transaction, &job).await?;
                changed += result.rows_affected();
                cursor = Some(row_id);
            }
        }
        transaction.commit().await?;
        Ok(changed)
    }

    pub async fn unread_message_ids(&self, conversation_id: &str) -> Result<Vec<String>, Error> {
        Ok(crate::db::query_scalar::<String>(
            "SELECT mention.message_id FROM message_mentions mention \
             INNER JOIN messages message ON message.message_id = mention.message_id \
             AND message.conversation_id = mention.conversation_id \
             WHERE mention.conversation_id = ? AND COALESCE(mention.has_read, FALSE) = FALSE \
             ORDER BY message.created_at ASC, message.rowid ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.0)
        .await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn parse_and_save_mention_data(
        &self,
        message_id: &str,
        conversation_id: &str,
        content: impl Into<Option<&str>>,
        sender_id: &str,
        quote_message: impl Into<Option<&QuoteMessage>>,
        current_user_id: &str,
        current_user_identity_number: &str,
    ) -> Result<(), Error> {
        let has_mention = parse_mention_data(
            content.into(),
            sender_id,
            quote_message.into(),
            current_user_id,
            current_user_identity_number,
        );
        if has_mention {
            self.insert_message_mention(MessageMention {
                message_id: message_id.to_string(),
                conversation_id: conversation_id.to_string(),
                has_read: false,
            })
            .await?;
        }
        Ok(())
    }

    pub async fn insert_message_mention(
        &self,
        message_mention: MessageMention,
    ) -> Result<(), Error> {
        crate::db::query(
            "INSERT OR REPLACE INTO message_mentions \
             (message_id, conversation_id, has_read) VALUES (?, ?, ?)",
        )
        .bind(message_mention.message_id)
        .bind(message_mention.conversation_id)
        .bind(message_mention.has_read)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete_message_mention(&self, message_id: &str) -> Result<u64, Error> {
        let result = crate::db::query("DELETE FROM message_mentions WHERE message_id = ?")
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn mark_mention_read(&self, ids: &[String]) -> Result<u64, Error> {
        if ids.is_empty() {
            return Ok(0);
        }

        let chunks = ids.chunks(MARK_LIMIT);
        let mut rows_affected: u64 = 0;
        for chunk in chunks {
            let query = format!(
                "UPDATE message_mentions SET has_read = true WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            let affected = crate::db::query(sqlx::AssertSqlSafe(query))
                .bind_list(chunk)
                .execute(&self.0)
                .await?
                .rows_affected();
            rows_affected += affected;
        }
        Ok(rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::mixin::MixinDatabase;

    async fn mentioned_database() -> (tempfile::TempDir, MixinDatabase) {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        for conversation in ["chosen", "other"] {
            crate::db::query(
                "INSERT INTO conversations (conversation_id, created_at, status) VALUES (?, 0, 0)",
            )
            .bind(conversation)
            .execute(&database.message_dao.0)
            .await
            .unwrap();
        }
        for (id, conversation) in [
            ("first", "chosen"),
            ("second", "chosen"),
            ("foreign", "other"),
        ] {
            database
                .message_dao
                .insert_message(&crate::db::mixin::message::Message {
                    message_id: id.into(),
                    conversation_id: conversation.into(),
                    user_id: "sender".into(),
                    category: "SIGNAL_TEXT".into(),
                    status: sdk::MessageStatus::Delivered,
                    created_at: chrono::Utc::now().naive_utc(),
                    ..Default::default()
                })
                .await
                .unwrap();
            database
                .message_mention_dao
                .insert_message_mention(MessageMention {
                    message_id: id.into(),
                    conversation_id: conversation.into(),
                    has_read: false,
                })
                .await
                .unwrap();
        }
        (directory, database)
    }

    #[tokio::test]
    async fn clearing_mentions_is_conversation_scoped_idempotent_and_enqueues_exact_acknowledgements(
    ) {
        let (_directory, database) = mentioned_database().await;
        let dao = &database.message_mention_dao;
        assert_eq!(
            dao.mark_read_and_enqueue("chosen", Some("foreign"))
                .await
                .unwrap(),
            0
        );
        assert_eq!(dao.mark_read_and_enqueue("chosen", None).await.unwrap(), 2);
        assert_eq!(dao.mark_read_and_enqueue("chosen", None).await.unwrap(), 0);
        assert!(dao.unread_message_ids("chosen").await.unwrap().is_empty());
        assert_eq!(dao.unread_message_ids("other").await.unwrap(), ["foreign"]);
        let jobs: Vec<(String, String)> = crate::db::query_as(
            "SELECT conversation_id, blaze_message FROM jobs WHERE action = 'CREATE_MESSAGE'",
        )
        .fetch_all(&dao.0)
        .await
        .unwrap();
        assert_eq!(jobs.len(), 2);
        let mut ids = Vec::new();
        for (conversation, payload) in jobs {
            assert_eq!(conversation, "chosen");
            let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
            assert_eq!(payload["status"], "MENTION_READ");
            ids.push(payload["message_id"].as_str().unwrap().to_owned());
        }
        ids.sort();
        assert_eq!(ids, ["first", "second"]);
    }

    #[tokio::test]
    async fn a_failed_acknowledgement_insert_rolls_back_mention_read_state() {
        let (_directory, database) = mentioned_database().await;
        let dao = &database.message_mention_dao;
        crate::db::query("CREATE TRIGGER reject_mention_job BEFORE INSERT ON jobs BEGIN SELECT RAISE(ABORT, 'rejected'); END")
            .execute(&dao.0).await.unwrap();
        assert!(dao
            .mark_read_and_enqueue("chosen", Some("first"))
            .await
            .is_err());
        assert_eq!(
            dao.unread_message_ids("chosen").await.unwrap(),
            ["first", "second"]
        );
        let count: i64 = crate::db::query_scalar("SELECT COUNT(*) FROM jobs")
            .fetch_one(&dao.0)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn parses_exact_numeric_mentions_only() {
        assert!(parse_mention_data(
            Some("hello @7000"),
            "sender",
            None,
            "me",
            "7000"
        ));
        assert!(!parse_mention_data(
            Some("hello @70001"),
            "sender",
            None,
            "me",
            "7000"
        ));
        assert!(!parse_mention_data(
            Some("hello @7000"),
            "me",
            None,
            "me",
            "7000"
        ));
    }

    #[tokio::test]
    async fn persists_marks_and_deletes_mentions_using_schema_columns() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let dao = database.message_mention_dao;
        dao.insert_message_mention(MessageMention {
            message_id: "message".to_string(),
            conversation_id: "conversation".to_string(),
            has_read: false,
        })
        .await
        .unwrap();

        assert_eq!(
            dao.mark_mention_read(&["message".to_string()])
                .await
                .unwrap(),
            1
        );
        let has_read: bool = crate::db::query_scalar(
            "SELECT has_read FROM message_mentions WHERE message_id = 'message'",
        )
        .fetch_one(&dao.0)
        .await
        .unwrap();
        assert!(has_read);
        assert_eq!(dao.delete_message_mention("message").await.unwrap(), 1);
    }
}

crate::db::storage::row_mapping!(MessageMention {
    message_id: "message_id",
    conversation_id: "conversation_id",
    has_read: "has_read",
});
