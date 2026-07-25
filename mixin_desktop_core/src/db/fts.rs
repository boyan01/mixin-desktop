use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

mod migration;

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
