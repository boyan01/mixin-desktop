use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub use auth::*;

use crate::db::Error;

pub mod auth;
pub struct AppDatabase {
    pub auth_dao: AuthDao,
}

impl AppDatabase {
    pub async fn connect() -> Result<Self, Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename("app.db")
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
        })
    }
}
