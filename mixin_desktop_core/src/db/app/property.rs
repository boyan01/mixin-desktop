use sqlx::{Error, Sqlite};

#[derive(Clone)]
pub struct PropertyDao(pub(crate) sqlx::Pool<Sqlite>);

impl PropertyDao {
    pub async fn get(&self, group: &str, key: &str) -> Result<Option<String>, Error> {
        sqlx::query_scalar("SELECT value FROM properties WHERE \"group\" = ? AND \"key\" = ?")
            .bind(group)
            .bind(key)
            .fetch_optional(&self.0)
            .await
    }

    pub async fn set(&self, group: &str, key: &str, value: &str) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT INTO properties ("group", "key", value) VALUES (?, ?, ?)
               ON CONFLICT("key", "group") DO UPDATE SET value = excluded.value"#,
        )
        .bind(group)
        .bind(key)
        .bind(value)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
