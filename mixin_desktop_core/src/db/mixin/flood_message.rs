use chrono::NaiveDateTime;

use crate::db::Error;

#[derive(Clone)]
pub struct FloodMessageDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(sqlx::FromRow)]
pub struct FloodMessage {
    pub message_id: String,
    pub data: String,
    pub created_at: NaiveDateTime,
}

impl FloodMessageDao {
    pub async fn insert_flood_message(&self, message: FloodMessage) -> Result<(), Error> {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO flood_messages (message_id, data, created_at) VALUES (?, ?, ?)",
        )
        .bind(message.message_id)
        .bind(message.data)
        .bind(message.created_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn flood_messages(&self) -> Result<Vec<FloodMessage>, Error> {
        let result = sqlx::query_as::<_, FloodMessage>(
            "SELECT * FROM flood_messages ORDER BY created_at ASC LIMIT 10",
        )
        .fetch_all(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn delete_flood_message(&self, m_id: &String) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM flood_messages WHERE message_id = ?")
            .bind(m_id)
            .execute(&self.0)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn latest_flood_message_created_at(&self) -> Result<Option<NaiveDateTime>, Error> {
        let latest = sqlx::query_scalar::<_, NaiveDateTime>(
            "SELECT created_at FROM flood_messages ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.0)
        .await?;
        Ok(latest)
    }
}
