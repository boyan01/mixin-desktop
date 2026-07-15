use sqlx::{QueryBuilder, Sqlite};

use crate::db::Error;

#[derive(Clone)]
pub struct MessageHistoryDao(pub(crate) sqlx::Pool<Sqlite>);

impl MessageHistoryDao {
    pub async fn insert(&self, message_id: &str) -> Result<(), Error> {
        sqlx::query("INSERT INTO messages_history (message_id) VALUES (?)")
            .bind(message_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn insert_list(&self, message_ids: &[String]) -> Result<(), Error> {
        if message_ids.is_empty() {
            return Ok(());
        }
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT INTO messages_history (message_id) VALUES ");

        query_builder.push_values(message_ids.iter(), |mut b, message_id| {
            b.push_bind(message_id);
        });
        let query = query_builder.build();
        query.execute(&self.0).await?;
        Ok(())
    }

    pub async fn exists(&self, message_id: &str) -> Result<bool, Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM messages_history WHERE message_id = ?)",
        )
        .bind(message_id)
        .fetch_one(&self.0)
        .await?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::MixinDatabase;

    #[tokio::test]
    async fn reports_persisted_message_history() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();

        assert!(!database
            .message_history_dao
            .exists("message")
            .await
            .unwrap());
        database.message_history_dao.insert_list(&[]).await.unwrap();
        database
            .message_history_dao
            .insert("message")
            .await
            .unwrap();
        assert!(database
            .message_history_dao
            .exists("message")
            .await
            .unwrap());
    }
}
