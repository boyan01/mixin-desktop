use std::fmt::{Display, Formatter};
use diesel::r2d2;

#[derive(Debug)]
pub enum Error {
    PoolError(String),
    DbError(diesel::result::Error),
}

impl From<r2d2::PoolError> for Error {
    fn from(value: r2d2::PoolError) -> Error {
        Error::PoolError(value.to_string())
    }
}

impl From<diesel::result::Error> for Error {
    fn from(value: diesel::result::Error) -> Self {
        Error::DbError(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "db error")
    }
}

impl std::error::Error for Error {}