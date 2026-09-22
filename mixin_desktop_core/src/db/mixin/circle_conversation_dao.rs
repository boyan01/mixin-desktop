use crate::db::QueryBuilder;

use crate::db::Error;

#[derive(Clone)]
pub struct CircleConversationDao(pub(crate) crate::db::Pool);

impl CircleConversationDao {
    pub async fn insert(&self, cs: &[sdk::CircleConversation]) -> Result<(), Error> {
        if cs.is_empty() {
            return Ok(());
        }
        let mut query_builder: QueryBuilder = QueryBuilder::new(
            "INSERT OR REPLACE INTO circle_conversations (circle_id, conversation_id, created_at) ",
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
        crate::db::query(
            "DELETE FROM circle_conversations WHERE circle_id = ? AND conversation_id = ?",
        )
        .bind(circle)
        .bind(conversation)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete_by_circle(&self, circle: &str) -> Result<(), Error> {
        crate::db::query("DELETE FROM circle_conversations WHERE circle_id = ?")
            .bind(circle)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
