use crate::db::mixin::message::QuoteMessage;
use crate::db::Error;

pub struct MessageMentionDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct MessageMention {
    pub message_mention_id: String,
    pub message_id: String,
    pub has_read: bool,
}

fn parse_mention_data(
    content: &Option<String>,
    sender_id: &str,
    quote_message: &Option<QuoteMessage>,
    current_user_id: &str,
    current_user_identity_number: &str,
) -> bool {
    if let Some(quote) = quote_message {
        if quote.user_id == current_user_id
            && quote.user_identity_number == current_user_identity_number
        {
            return true;
        }
    }
    if let Some(content) = content {
        if sender_id != current_user_id
            && content.contains(&format!("@{}", current_user_identity_number))
        {
            return true;
        }
    }
    false
}

impl MessageMentionDao {
    #[allow(clippy::too_many_arguments)]
    pub async fn parse_and_save_mention_data(
        &self,
        message_id: &str,
        conversation_id: &str,
        content: &Option<String>,
        sender_id: &str,
        quote_message: &Option<QuoteMessage>,
        current_user_id: &str,
        current_user_identity_number: &str,
    ) -> Result<(), Error> {
        let has_mention = parse_mention_data(
            content,
            sender_id,
            quote_message,
            current_user_id,
            current_user_identity_number,
        );
        if has_mention {
            self.insert_message_mention(MessageMention {
                message_mention_id: message_id.to_string(),
                message_id: conversation_id.to_string(),
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
        let _ = sqlx::query(
            "INSERT INTO message_mentions (message_mention_id, message_id, has_read) VALUES (?, ?, ?)",
        )
            .bind(message_mention.message_mention_id)
            .bind(message_mention.message_id)
            .bind(message_mention.has_read)
            .execute(&self.0)
            .await?;
        Ok(())
    }
    pub async fn delete_message_mention(&self, message_id: &String) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM message_mentions WHERE message_mention_id = ?")
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }
}
