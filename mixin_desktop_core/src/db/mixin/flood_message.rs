use std::sync::Arc;

use chrono::NaiveDateTime;
use tokio::sync::Notify;

use crate::db::datetime::DatabaseDateTime;
use crate::db::Error;

#[derive(Clone)]
pub struct FloodMessageDao {
    pool: sqlx::Pool<sqlx::Sqlite>,
    notify: Arc<Notify>,
}

#[derive(sqlx::FromRow)]
pub struct FloodMessage {
    pub message_id: String,
    pub data: String,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: NaiveDateTime,
}

impl FloodMessageDao {
    pub fn new(pool: sqlx::Pool<sqlx::Sqlite>) -> Self {
        Self {
            pool,
            notify: Arc::new(Notify::new()),
        }
    }

    pub async fn insert_flood_message(&self, message: FloodMessage) -> Result<(), Error> {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO flood_messages (message_id, data, created_at) VALUES (?, ?, ?)",
        )
        .bind(message.message_id)
        .bind(message.data)
        .bind(message.created_at.and_utc().timestamp_millis())
        .execute(&self.pool)
        .await?;
        self.notify.notify_one();
        Ok(())
    }

    pub async fn wait_for_message(&self) {
        self.notify.notified().await;
    }

    pub async fn flood_messages(&self) -> Result<Vec<FloodMessage>, Error> {
        let result = sqlx::query_as::<_, FloodMessage>(
            "SELECT * FROM flood_messages ORDER BY created_at ASC LIMIT 10",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(result)
    }

    pub async fn delete_flood_message(&self, m_id: &String) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM flood_messages WHERE message_id = ?")
            .bind(m_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn latest_flood_message_created_at(&self) -> Result<Option<NaiveDateTime>, Error> {
        let latest = sqlx::query_scalar::<_, DatabaseDateTime>(
            "SELECT created_at FROM flood_messages ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(latest.map(Into::into))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;

    use super::FloodMessage;
    use crate::db::mixin::MixinDatabase;

    #[tokio::test]
    async fn insert_notifies_waiting_consumer() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let waiting_dao = database.flood_message_dao.clone();
        let inserting_dao = database.flood_message_dao.clone();

        let waiter = waiting_dao.wait_for_message();
        tokio::pin!(waiter);
        inserting_dao
            .insert_flood_message(FloodMessage {
                message_id: "message-id".to_string(),
                data: "{}".to_string(),
                created_at: Utc::now().naive_utc(),
            })
            .await
            .unwrap();

        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .unwrap();
        assert_eq!(inserting_dao.flood_messages().await.unwrap().len(), 1);
    }
}
