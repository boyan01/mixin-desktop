use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    DbError(sqlx::Error),
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Error {
        Error::DbError(value)
    }
}


impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "db error")
    }
}

impl std::error::Error for Error {}