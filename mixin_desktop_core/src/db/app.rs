use anyhow::{bail, Context};
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
        migrate(&pool)
            .await
            .with_context(|| "app database migration failed")?;
        Ok(AppDatabase {
            auth_dao: AuthDao(pool.clone()),
            property_dao: PropertyDao(pool.clone()),
        })
    }
}

const SCHEMA_VERSION: i64 = 1;

async fn migrate(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    let version = crate::db::migration::user_version(pool).await?;
    if version > SCHEMA_VERSION {
        bail!("app database version {version} is newer than supported {SCHEMA_VERSION}");
    }
    if version == SCHEMA_VERSION {
        return Ok(());
    }
    if crate::db::migration::has_application_tables(pool).await? {
        bail!("app database has no Drift user_version");
    }

    let mut transaction = pool.begin().await?;
    sqlx::raw_sql(include_str!("app/schema.sql"))
        .execute(&mut *transaction)
        .await?;
    sqlx::query("PRAGMA user_version = 1")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn schema_matches_flutter_app() {
        let path =
            std::env::temp_dir().join(format!("mixin-desktop-app-{}.db", uuid::Uuid::new_v4()));
        let database = AppDatabase::connect_at(&path).await.unwrap();

        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&database.auth_dao.0)
            .await
            .unwrap();
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&database.auth_dao.0)
        .await
        .unwrap();

        assert_eq!(version, 1);
        assert_eq!(tables, vec!["properties"]);

        drop(database);
        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_file(path.with_extension("db-shm")).await;
        let _ = tokio::fs::remove_file(path.with_extension("db-wal")).await;
    }
}
