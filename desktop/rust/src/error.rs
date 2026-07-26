use mixin_desktop_api::ClientError;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum SwiftClientError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("operation cancelled")]
    Cancelled,
    #[error("{message}")]
    InvalidArgument { message: String },
    #[error("{message}")]
    Internal { message: String },
}

impl From<ClientError> for SwiftClientError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::Unauthorized => Self::Unauthorized,
            ClientError::NotFound => Self::NotFound,
            ClientError::Cancelled => Self::Cancelled,
            ClientError::InvalidArgument(message) => Self::InvalidArgument { message },
            ClientError::Internal(message) => Self::Internal { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use mixin_desktop_api::ClientError;

    use super::SwiftClientError;

    #[test]
    fn invalid_argument_preserves_message_for_swift() {
        let error = SwiftClientError::from(ClientError::InvalidArgument(
            "unsupported proxy type".to_string(),
        ));

        assert!(matches!(
            error,
            SwiftClientError::InvalidArgument { message }
                if message == "unsupported proxy type"
        ));
    }
}
