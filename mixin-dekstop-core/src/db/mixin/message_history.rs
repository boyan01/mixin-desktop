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
