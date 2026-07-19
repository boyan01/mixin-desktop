use crate::db::mixin::database::MARK_LIMIT;
use crate::db::mixin::util::{expand_var, BindListForQuery};
use crate::db::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct PinMessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PinMessage {
    pub message_id: String,
    pub conversation_id: String,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PinMessageMinimal {
    #[serde(rename = "category")]
    pub category: String,
    pub message_id: String,
    pub content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::PinMessageMinimal;

    #[test]
    fn pin_message_minimal_uses_flutter_category_key() {
        let message = PinMessageMinimal {
            category: "PLAIN_TEXT".to_string(),
            message_id: "message-id".to_string(),
            content: Some("hello".to_string()),
        };

        let json = serde_json::to_value(message).unwrap();
        assert_eq!(json["category"], "PLAIN_TEXT");
        assert!(json.get("type").is_none());
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct PinMessagePreview {
    pub message_id: String,
    pub content: Option<String>,
    pub sender_name: Option<String>,
}

impl PinMessageDao {
    pub async fn message_ids(&self, conversation_id: &str) -> Result<Vec<String>, Error> {
        Ok(sqlx::query_scalar(
            r#"
SELECT pin.message_id
FROM pin_messages pin INDEXED BY index_pin_messages_conversation_id
INNER JOIN messages message ON message.message_id = pin.message_id
WHERE pin.conversation_id = ?
ORDER BY message.created_at DESC, message.message_id DESC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn latest_preview(
        &self,
        conversation_id: &str,
    ) -> Result<Option<PinMessagePreview>, Error> {
        Ok(sqlx::query_as::<_, PinMessagePreview>(
            r#"
SELECT pin.message_id,
       pin_event.content,
       sender.full_name AS sender_name
FROM pin_messages pin
INNER JOIN messages pinned_message ON pinned_message.message_id = pin.message_id
LEFT JOIN messages pin_event
       ON pin_event.conversation_id = pin.conversation_id
      AND pin_event.category = 'MESSAGE_PIN'
      AND pin_event.quote_message_id = pin.message_id
LEFT JOIN users sender ON sender.user_id = pin_event.user_id
WHERE pin.conversation_id = ?
ORDER BY pinned_message.created_at DESC,
         pin_event.created_at DESC
LIMIT 1
            "#,
        )
        .bind(conversation_id)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn delete_pin_message(&self, message_id: &[String]) -> Result<(), Error> {
        for chunk in message_id.chunks(MARK_LIMIT) {
            let query = format!(
                "DELETE FROM pin_messages WHERE message_id IN ({})",
                expand_var(chunk.len())
            );
            sqlx::query(sqlx::AssertSqlSafe(query))
                .bind_list(chunk)
                .execute(&self.0)
                .await?;
        }
        Ok(())
    }

    pub async fn insert_pin_message(&self, pin_message: &PinMessage) -> Result<(), Error> {
        let _ = sqlx::query(
            "INSERT INTO pin_messages (message_id, conversation_id, created_at) VALUES (?, ?, ?)",
        )
        .bind(&pin_message.message_id)
        .bind(&pin_message.conversation_id)
        .bind(pin_message.created_at.timestamp_millis())
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
