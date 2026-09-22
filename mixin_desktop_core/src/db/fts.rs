use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

mod migration;

#[cfg(not(target_arch = "wasm32"))]
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
    migration::MIGRATOR.migrate(&pool).await?;
    pool.close().await;
    Ok(())
}

/// wasm: open through the Worker bridge; OPFS directories and storage-mode
/// pragmas are owned by the JS open handler.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn migrate(path: &Path) -> anyhow::Result<()> {
    let pool = crate::db::Pool::open(&path.to_string_lossy()).await?;
    migration::MIGRATOR.migrate(&pool).await?;
    pool.close().await;
    Ok(())
}
