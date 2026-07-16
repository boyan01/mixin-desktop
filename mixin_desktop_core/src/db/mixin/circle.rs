use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};

use crate::db::Error;

#[derive(Clone)]
pub struct CircleDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Circle {
    pub circle_id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub ordered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CircleSummary {
    pub circle_id: String,
    pub name: String,
    pub conversation_count: i64,
}

impl CircleDao {
    pub async fn summaries(&self) -> Result<Vec<CircleSummary>, Error> {
        Ok(sqlx::query_as::<_, CircleSummary>(
            r#"SELECT circle.circle_id, circle.name,
                      COUNT(circle_conversation.conversation_id) AS conversation_count
               FROM circles circle
               LEFT JOIN circle_conversations circle_conversation
                 ON circle_conversation.circle_id = circle.circle_id
               GROUP BY circle.circle_id, circle.name, circle.ordered_at, circle.created_at
               ORDER BY circle.ordered_at, circle.created_at"#,
        )
        .fetch_all(&self.0)
        .await?)
    }

    pub async fn list(&self) -> Result<Vec<Circle>, Error> {
        Ok(
            sqlx::query_as::<_, Circle>("SELECT * FROM circles ORDER BY ordered_at, created_at")
                .fetch_all(&self.0)
                .await?,
        )
    }

    pub async fn insert_circles(&self, circles: &[sdk::Circle]) -> Result<(), Error> {
        if circles.is_empty() {
            return Ok(());
        }
        let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "INSERT OR REPLACE INTO circles (circle_id, name, created_at) VALUES ",
        );
        query_builder.push_values(circles.iter(), |mut b, circle| {
            b.push_bind(&circle.circle_id)
                .push_bind(&circle.name)
                .push_bind(circle.created_at);
        });
        let query = query_builder.build();
        query.execute(&self.0).await?;
        Ok(())
    }

    pub async fn exists(&self, circle_id: &str) -> Result<bool, Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM circles WHERE circle_id = ?)",
        )
        .bind(circle_id)
        .fetch_one(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn delete(&self, id: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM circles WHERE circle_id = ?")
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
