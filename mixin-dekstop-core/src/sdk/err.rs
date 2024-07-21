use std::fmt::{Debug, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ApiError {
    Server(Error),
    Request(reqwest::Error),
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    status: i64,
    code: i64,
    description: String,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Server(err) => f.write_fmt(format_args!(
                "{}, status: {} code: {}",
                err.description, err.status, err.code
            )),
            ApiError::Request(err) => {
                write!(f, "request err: {err}")
            }
            ApiError::Unknown(err) => {
                write!(f, "unknown err: {err}")
            }
        }
    }
}

impl From<Error> for ApiError {
    fn from(value: Error) -> Self {
        ApiError::Server(value)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        ApiError::Request(value)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        ApiError::Unknown(value.to_string())
    }
}

impl From<String> for ApiError {
    fn from(value: String) -> Self {
        ApiError::Unknown(value)
    }
}
