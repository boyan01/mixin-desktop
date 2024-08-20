use sqlx::QueryBuilder;

use crate::db::Error;

#[derive(Clone)]
pub struct CircleConversationDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl CircleConversationDao {
    pub async fn insert(&self, cs: &[sdk::CircleConversation]) -> Result<(), Error> {
        let mut query_builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO circle_conversations (circle_id, conversation_id, created_at) VALUES ",
        );
        query_builder.push_values(cs.iter(), |mut b, c| {
            b.push_bind(&c.circle_id)
                .push_bind(&c.conversation_id)
                .push_bind(c.created_at);
        });
        let query = query_builder.build();
        query.execute(&self.0).await?;
        Ok(())
    }

    pub async fn delete(&self, circle: &str, conversation: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM circle_conversations WHERE circle_id = ? AND conversation_id = ?")
            .bind(circle)
            .bind(conversation)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn delete_by_circle(&self, circle: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM circle_conversations WHERE circle_id = ?")
            .bind(circle)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
