use sqlx::{Pool, Sqlite};

pub(crate) async fn user_version(pool: &Pool<Sqlite>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
}

pub(crate) async fn has_application_tables(pool: &Pool<Sqlite>) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%')",
    )
    .fetch_one(pool)
    .await
}
