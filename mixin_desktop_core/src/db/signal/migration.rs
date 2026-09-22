#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use futures::FutureExt;
#[cfg(not(target_arch = "wasm32"))]
use log::info;
#[cfg(not(target_arch = "wasm32"))]
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::db::migration::{Migration, MigrationFuture, Migrator};

const MIGRATIONS: &[Migration] = &[Migration::action(
    2,
    "add and migrate signal properties",
    migrate_to_v2,
)];

pub(super) const SCHEMA_VERSION: i64 = 2;
pub(super) const MIGRATOR: Migrator = Migrator::new(
    "signal",
    SCHEMA_VERSION,
    include_str!("schema.sql"),
    MIGRATIONS,
);

/// Copies counters out of the legacy desktop app database. The legacy app
/// database only exists for native upgrades; wasm databases are created
/// directly at the current schema, so this migration never runs there and
/// has nothing to copy.
#[cfg(target_arch = "wasm32")]
fn migrate_to_v2(_connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    Box::pin(async { Ok(()) })
}

#[cfg(not(target_arch = "wasm32"))]
fn migrate_to_v2(connection: &mut crate::db::Conn) -> MigrationFuture<'_> {
    async move {
        crate::db::query(
            "CREATE TABLE properties (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            )",
        )
        .execute(&mut *connection)
        .await?;

        let (_, _, signal_path): (i64, String, String) =
            crate::db::query_as("PRAGMA database_list")
                .fetch_one(&mut *connection)
                .await?;
        let signal_path = Path::new(&signal_path);
        let Some(account_directory) = signal_path.parent() else {
            return Ok(());
        };
        let Some(identity_number) = account_directory
            .file_name()
            .and_then(|value| value.to_str())
        else {
            return Ok(());
        };
        let Some(data_directory) = account_directory.parent() else {
            return Ok(());
        };
        let app_path = data_directory.join("app.db");
        if !tokio::fs::try_exists(&app_path).await? {
            return Ok(());
        }

        let app_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(app_path)
                    .read_only(true),
            )
            .await?;
        let properties: Vec<(String, String)> = crate::db::query_as(
            r#"SELECT "key", value
               FROM properties
               WHERE "group" = ?
                 AND "key" IN (
                     'next_pre_key_id',
                     'next_signed_pre_key_id',
                     'has_push_signal_keys'
                 )"#,
        )
        .bind(format!("crypto:{identity_number}"))
        .fetch_all(&app_pool)
        .await?;
        app_pool.close().await;

        for (key, value) in &properties {
            crate::db::query("INSERT INTO properties (key, value) VALUES (?, ?)")
                .bind(key)
                .bind(value)
                .execute(&mut *connection)
                .await?;
        }
        if !properties.is_empty() {
            info!(
                "migrated {} legacy signal properties from app database",
                properties.len()
            );
        }
        Ok(())
    }
    .boxed()
}
