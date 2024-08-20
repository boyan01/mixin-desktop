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

impl CircleDao {
    pub async fn insert_circles(&self, circles: &[sdk::Circle]) -> Result<(), Error> {
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
