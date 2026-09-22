pub use error::Error;
pub use mixin::MixinDatabase;
pub use signal::database::SignalDatabase;

mod datetime;
pub mod error;
pub mod fts;

pub mod app;
mod migration;
pub mod mixin;
pub mod path;
pub mod signal;

pub mod storage;
pub use storage::{query, query_as, query_scalar, raw_sql, Executor, QueryBuilder};
#[cfg(not(target_arch = "wasm32"))]
pub type Pool = sqlx::SqlitePool;
#[cfg(not(target_arch = "wasm32"))]
pub type Conn = sqlx::SqliteConnection;
#[cfg(not(target_arch = "wasm32"))]
pub type Transaction<'a> = sqlx::Transaction<'a, sqlx::Sqlite>;
#[cfg(target_arch = "wasm32")]
pub use storage::browser::{Conn, Pool, Transaction};
