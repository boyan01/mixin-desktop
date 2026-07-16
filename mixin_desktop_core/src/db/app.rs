use anyhow::Context;
use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

pub use auth::*;
pub use property::*;

use crate::db::Error;

pub mod auth;
pub mod property;
pub struct AppDatabase {
    pub auth_dao: AuthDao,
    pub property_dao: PropertyDao,
}

impl AppDatabase {
    pub async fn connect() -> Result<Self, Error> {
        let path = crate::db::path::app_database_path("app.db")?;
        Self::connect_at(path).await
    }

    pub async fn connect_at(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        crate::db::path::create_parent_directory(path).await?;
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(path)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal)
                    .foreign_keys(true)
                    .create_if_missing(true),
            )
            .await?;
        let migrator = sqlx::migrate!("./src/db/app/migrations");
        migrator
            .run(&pool)
            .await
            .with_context(|| "migrations failed")?;
        Ok(AppDatabase {
            auth_dao: AuthDao(pool.clone()),
            property_dao: PropertyDao(pool.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::Row;

    use super::*;

    #[tokio::test]
    async fn auth_schema_persists_primary_session_id() {
        let path =
            std::env::temp_dir().join(format!("mixin-desktop-app-{}.db", uuid::Uuid::new_v4()));
        let database = AppDatabase::connect_at(&path).await.unwrap();

        let columns = sqlx::query("PRAGMA table_info(auths)")
            .fetch_all(&database.auth_dao.0)
            .await
            .unwrap();
        let names = columns
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect::<Vec<_>>();

        assert!(names.iter().any(|name| name == "primary_session_id"));

        drop(database);
        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_file(path.with_extension("db-shm")).await;
        let _ = tokio::fs::remove_file(path.with_extension("db-wal")).await;
    }
}
