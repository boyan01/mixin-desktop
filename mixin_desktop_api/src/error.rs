use mixin_desktop_core::CoreError;

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("operation cancelled")]
    Cancelled,
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    Internal(String),
}

impl From<CoreError> for ClientError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::NotFound => Self::NotFound,
            CoreError::Other { message } => Self::Internal(message),
        }
    }
}

impl From<anyhow::Error> for ClientError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<mixin_desktop_media::MediaError> for ClientError {
    fn from(error: mixin_desktop_media::MediaError) -> Self {
        Self::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientError, CoreError};

    #[test]
    fn core_not_found_maps_to_public_not_found() {
        assert!(matches!(
            ClientError::from(CoreError::NotFound),
            ClientError::NotFound
        ));
    }

    #[test]
    fn internal_core_error_maps_to_public_internal_message() {
        let error = ClientError::from(CoreError::Other {
            message: "database unavailable".to_string(),
        });

        assert!(matches!(
            error,
            ClientError::Internal(message) if message == "database unavailable"
        ));
    }
}
