use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("server error: {0}")]
    Server(Error),
    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error("failed to serialize json: {0}")]
    JsonSerializeError(#[from] serde_json::Error),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    status: i64,
    code: i64,
    description: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error: status: {}, code: {}, description: {}",
            self.status, self.code, self.description
        )
    }
}
