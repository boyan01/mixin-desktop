use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::message::QuoteMessage;
use crate::db::mixin::util::{expand_var, BindListForQuery};
use crate::db::Error;

#[derive(Clone)]
pub struct MessageMentionDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
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
    pub async fn unread_message_ids(&self, conversation_id: &str) -> Result<Vec<String>, Error> {
        Ok(sqlx::query_scalar::<_, String>(
            "SELECT message_id FROM message_mentions \
             WHERE conversation_id = ? AND COALESCE(has_read, FALSE) = FALSE",
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
        sqlx::query(
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
        let result = sqlx::query("DELETE FROM message_mentions WHERE message_id = ?")
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
            let affected = sqlx::query(sqlx::AssertSqlSafe(query))
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
        let has_read: bool = sqlx::query_scalar(
            "SELECT has_read FROM message_mentions WHERE message_id = 'message'",
        )
        .fetch_one(&dao.0)
        .await
        .unwrap();
        assert!(has_read);
        assert_eq!(dao.delete_message_mention("message").await.unwrap(), 1);
    }
}
