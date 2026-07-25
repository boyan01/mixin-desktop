use sqlx::{Error, Sqlite};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyGroup {
    Account,
    Auth,
    Setting,
}

impl PropertyGroup {
    fn value(&self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Auth => "auth",
            Self::Setting => "setting",
        }
    }
}

#[derive(Clone)]
pub struct PropertyDao(pub(crate) sqlx::Pool<Sqlite>);

impl PropertyDao {
    pub async fn get(&self, group: PropertyGroup, key: &str) -> Result<Option<String>, Error> {
        sqlx::query_scalar("SELECT value FROM properties WHERE \"group\" = ? AND \"key\" = ?")
            .bind(group.value())
            .bind(key)
            .fetch_optional(&self.0)
            .await
    }

    pub async fn set(&self, group: PropertyGroup, key: &str, value: &str) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT INTO properties ("group", "key", value) VALUES (?, ?, ?)
               ON CONFLICT("key", "group") DO UPDATE SET value = excluded.value"#,
        )
        .bind(group.value())
        .bind(key)
        .bind(value)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn update(
        &self,
        values: &[(PropertyGroup, &str, Option<&str>)],
    ) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        for (group, key, value) in values {
            let group = group.value();
            if let Some(value) = value {
                sqlx::query(
                    r#"INSERT INTO properties ("group", "key", value) VALUES (?, ?, ?)
                       ON CONFLICT("key", "group") DO UPDATE SET value = excluded.value"#,
                )
                .bind(group)
                .bind(key)
                .bind(value)
                .execute(&mut *transaction)
                .await?;
            } else {
                sqlx::query("DELETE FROM properties WHERE \"group\" = ? AND \"key\" = ?")
                    .bind(group)
                    .bind(key)
                    .execute(&mut *transaction)
                    .await?;
            }
        }
        transaction.commit().await?;
        Ok(())
    }
}
