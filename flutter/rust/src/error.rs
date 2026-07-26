pub type Result<T, E = CoreError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
#[flutter_rust_bridge::frb(non_opaque)]
/// flutter_rust_bridge:non_opaque
pub enum CoreError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("operation cancelled")]
    Cancelled,
    #[error("{message}")]
    InvalidArgument { message: String },
    #[error("{message}")]
    Other { message: String },
}

impl From<mixin_desktop_api::ClientError> for CoreError {
    fn from(error: mixin_desktop_api::ClientError) -> Self {
        match error {
            mixin_desktop_api::ClientError::Unauthorized => Self::Unauthorized,
            mixin_desktop_api::ClientError::NotFound => Self::NotFound,
            mixin_desktop_api::ClientError::Cancelled => Self::Cancelled,
            mixin_desktop_api::ClientError::InvalidArgument(message) => {
                Self::InvalidArgument { message }
            }
            mixin_desktop_api::ClientError::Internal(message) => Self::Other { message },
        }
    }
}

pub type Error = CoreError;

#[cfg(test)]
mod tests {
    use mixin_desktop_api::ClientError;

    use super::CoreError;

    #[test]
    fn cancelled_client_error_remains_typed_for_flutter() {
        assert!(matches!(
            CoreError::from(ClientError::Cancelled),
            CoreError::Cancelled
        ));
    }
}
