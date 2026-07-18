pub use database::MixinDatabase;

pub mod database;

pub mod app;
pub mod asset;
pub mod circle;
pub mod circle_conversation_dao;
pub mod conversation;
pub mod expired_message;
pub mod favorite_app;
pub mod fiat;
pub mod flood_message;
pub mod inscription;
pub mod job;
pub mod message;
pub mod message_fts;
pub mod message_history;
pub mod message_mention;
pub mod offset;
pub mod participant;
pub mod participant_session;
pub mod pin_message;
pub mod safe_snapshot;
pub mod snapshot;
pub mod sticker;
pub mod transcript_message;
pub mod user;
mod util;

#[cfg(test)]
mod tests {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    #[tokio::test]
    async fn migrations_revert_all_application_tables() {
        let directory = tempfile::tempdir().unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(directory.path().join("mixin.db"))
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let migrator = sqlx::migrate!("./src/db/mixin/migrations");

        migrator.run(&pool).await.unwrap();
        migrator.undo(&pool, 0).await.unwrap();

        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations'",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(tables.is_empty(), "tables left after rollback: {tables:?}");
    }
}
