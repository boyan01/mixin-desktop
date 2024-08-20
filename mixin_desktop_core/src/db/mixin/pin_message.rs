use crate::db::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct PinMessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PinMessage {
    pub message_id: String,
    pub conversation_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PinMessageMinimal {
    #[serde(rename = "type")]
    pub category: String,
    pub message_id: String,
    pub content: Option<String>,
}

impl PinMessageDao {
    pub async fn delete_pin_message(&self, message_id: &[String]) -> Result<(), Error> {
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("DELETE FROM pin_messages WHERE message_id IN (");
        for message_id in message_id {
            query_builder.push_bind(message_id);
            query_builder.push(',');
        }
        query_builder.push(')');
        let _ = query_builder.build().execute(&self.0).await?;
        Ok(())
    }

    pub async fn insert_pin_message(&self, pin_message: &PinMessage) -> Result<(), Error> {
        let _ = sqlx::query(
            "INSERT INTO pin_messages (message_id, conversation_id, created_at) VALUES (?, ?, ?)",
        )
        .bind(&pin_message.message_id)
        .bind(&pin_message.conversation_id)
        .bind(pin_message.created_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
