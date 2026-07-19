use std::path::Path;

use anyhow::bail;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

const SCHEMA_VERSION: i64 = 1;

pub(crate) async fn migrate(path: &Path) -> anyhow::Result<()> {
    crate::db::path::create_parent_directory(path).await?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .foreign_keys(true)
                .create_if_missing(true),
        )
        .await?;
    let version = crate::db::migration::user_version(&pool).await?;
    if version > SCHEMA_VERSION {
        bail!("fts database version {version} is newer than supported {SCHEMA_VERSION}");
    }
    if version == SCHEMA_VERSION {
        pool.close().await;
        return Ok(());
    }
    if crate::db::migration::has_application_tables(&pool).await? {
        bail!("fts database has no Drift user_version");
    }

    let mut transaction = pool.begin().await?;
    sqlx::raw_sql(include_str!("fts/schema.sql"))
        .execute(&mut *transaction)
        .await?;
    sqlx::query("PRAGMA user_version = 1")
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    pool.close().await;
    Ok(())
}
