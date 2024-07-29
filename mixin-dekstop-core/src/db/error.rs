#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("db error: {0}")]
    DbError(#[from] sqlx::Error),
    #[error("unknown error: {0:?}")]
    Other(#[from] anyhow::Error),
}
