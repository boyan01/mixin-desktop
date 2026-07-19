use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, Sqlite};

use crate::db::Error;

#[derive(Clone)]
pub struct CircleDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Circle {
    pub circle_id: String,
    pub name: String,
    #[sqlx(try_from = "crate::db::datetime::DatabaseDateTime")]
    pub created_at: DateTime<Utc>,
    #[sqlx(try_from = "crate::db::datetime::OptionalDatabaseDateTime")]
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
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT OR REPLACE INTO circles (circle_id, name, created_at) ");
        query_builder.push_values(circles.iter(), |mut b, circle| {
            b.push_bind(&circle.circle_id)
                .push_bind(&circle.name)
                .push_bind(circle.created_at.timestamp_millis());
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

    pub async fn update_name(&self, id: &str, name: &str) -> Result<(), Error> {
        sqlx::query("UPDATE circles SET name = ? WHERE circle_id = ?")
            .bind(name)
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub async fn update_orders(&self, ids: &[String]) -> Result<(), Error> {
        let now = Utc::now();
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        for (index, id) in ids.iter().enumerate() {
            sqlx::query("UPDATE circles SET ordered_at = ? WHERE circle_id = ?")
                .bind((now + Duration::milliseconds(index as i64)).timestamp_millis())
                .bind(id)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM circles WHERE circle_id = ?")
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::db::mixin::database::MixinDatabase;

    #[tokio::test]
    async fn updates_circle_name_and_order() {
        let directory = tempfile::tempdir().unwrap();
        let database = MixinDatabase::connect_at(directory.path().join("mixin.db"))
            .await
            .unwrap();
        let created_at = Utc::now();
        database
            .circle_dao
            .insert_circles(&[
                sdk::Circle {
                    circle_id: "one".to_string(),
                    name: "One".to_string(),
                    created_at,
                },
                sdk::Circle {
                    circle_id: "two".to_string(),
                    name: "Two".to_string(),
                    created_at: created_at + Duration::seconds(1),
                },
                sdk::Circle {
                    circle_id: "three".to_string(),
                    name: "Three".to_string(),
                    created_at: created_at + Duration::seconds(2),
                },
            ])
            .await
            .unwrap();

        database
            .circle_dao
            .update_name("two", "Renamed")
            .await
            .unwrap();
        database
            .circle_dao
            .update_orders(&["three".to_string(), "one".to_string(), "two".to_string()])
            .await
            .unwrap();

        let circles = database.circle_dao.list().await.unwrap();
        assert_eq!(
            circles
                .iter()
                .map(|circle| circle.circle_id.as_str())
                .collect::<Vec<_>>(),
            ["three", "one", "two"]
        );
        assert_eq!(circles[2].name, "Renamed");
    }
}
