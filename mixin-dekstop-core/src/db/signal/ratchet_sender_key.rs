pub struct RatchetSenderKeyDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(sqlx::FromRow, Debug)]
pub struct RatchetSenderKey {
    pub group_id: String,
    pub sender_id: String,
    pub status: String,
    pub message_id: Option<String>,
    pub created_at: String,
}

pub mod ratchet_sender_key_status {
    pub const REQUESTING: &str = "REQUESTING";
}

impl RatchetSenderKeyDao {
    pub async fn insert_sender_key(&self, key: &RatchetSenderKey) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO ratchet_sender_keys (group_id, sender_id, status, message_id, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&key.group_id)
        .bind(&key.sender_id)
        .bind(&key.status)
        .bind(&key.message_id)
        .bind(&key.created_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, group_id: &str, sender_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM ratchet_sender_keys WHERE group_id = ? AND sender_id = ?")
            .bind(group_id)
            .bind(sender_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn find_status(
        &self,
        group_id: &str,
        sender_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let result = sqlx::query_scalar::<_, String>(
            "SELECT status FROM ratchet_sender_keys WHERE group_id = ? AND sender_id = ?",
        )
        .bind(group_id)
        .bind(sender_id)
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }
}
