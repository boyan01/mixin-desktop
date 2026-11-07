use anyhow::Context;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

pub use auth::*;
pub use property::*;
pub use settings::*;

use crate::db::Error;

pub mod auth;
mod migration;
pub mod property;
mod settings;
#[cfg(not(target_arch = "wasm32"))]
mod shared_settings;
/// Process-scoped storage. `app.db` keeps NSH-only preferences; native
/// credentials and shared preferences live in the Flutter app's Hive boxes
/// next to it, whose lock this database holds while open.
pub struct AppDatabase {
    pub auth_dao: AuthDao,
    pub property_dao: PropertyDao,
    pub setting_dao: SettingDao,
}

impl AppDatabase {
    pub async fn connect() -> Result<Self, Error> {
        let path = crate::db::path::app_database_path("app.db")?;
        Self::connect_at(path).await
    }

    #[cfg(not(target_arch = "wasm32"))]
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
        migration::MIGRATOR
            .migrate(&pool)
            .await
            .with_context(|| "app database migration failed")?;
        let root = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("app database has no directory"))?
            .to_path_buf();
        let hydrated = crate::db::hive::hydrated_box(&root)?;
        let property_dao = PropertyDao(pool);
        let auth_dao = AuthDao::new(root, hydrated.clone());
        Ok(Self {
            setting_dao: SettingDao::new(
                property_dao.clone(),
                shared_settings::SharedSettings::new(hydrated, auth_dao.clone()),
            ),
            auth_dao,
            property_dao,
        })
    }

    /// wasm: open through the Worker bridge; OPFS directory creation and
    /// storage-mode pragmas are owned by the JS open handler.
    #[cfg(target_arch = "wasm32")]
    pub async fn connect_at(path: impl AsRef<Path>) -> Result<Self, Error> {
        let pool = crate::db::Pool::open(&path.as_ref().to_string_lossy()).await?;
        migration::MIGRATOR
            .migrate(&pool)
            .await
            .with_context(|| "app database migration failed")?;
        let property_dao = PropertyDao(pool);
        Ok(Self {
            auth_dao: AuthDao::new(property_dao.clone()),
            setting_dao: SettingDao::new(property_dao.clone()),
            property_dao,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn schema_matches_flutter_app() {
        let path =
            std::env::temp_dir().join(format!("mixin-desktop-app-{}.db", uuid::Uuid::new_v4()));
        let database = AppDatabase::connect_at(&path).await.unwrap();

        let version: i64 = crate::db::query_scalar("PRAGMA user_version")
            .fetch_one(&database.property_dao.0)
            .await
            .unwrap();
        let tables: Vec<String> = crate::db::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&database.property_dao.0)
        .await
        .unwrap();

        assert_eq!(version, migration::SCHEMA_VERSION);
        assert_eq!(tables, vec!["properties"]);

        drop(database);
        let _ = tokio::fs::remove_file(&path).await;
        let _ = tokio::fs::remove_file(path.with_extension("db-shm")).await;
        let _ = tokio::fs::remove_file(path.with_extension("db-wal")).await;
    }
}
