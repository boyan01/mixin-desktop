use mixin_desktop_core::CoreError;

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("permission denied")]
    PermissionDenied,
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("authorization timed out")]
    TimedOut,
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
            CoreError::PermissionDenied => Self::PermissionDenied,
            CoreError::TimedOut => Self::TimedOut,
            CoreError::Other { message } => Self::Internal(message),
        }
    }
}

impl From<anyhow::Error> for ClientError {
    fn from(error: anyhow::Error) -> Self {
        match error.downcast::<CoreError>() {
            Ok(error) => error.into(),
            Err(error) => Self::Internal(error.to_string()),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<mixin_desktop_media::MediaError> for ClientError {
    fn from(error: mixin_desktop_media::MediaError) -> Self {
        Self::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientError, CoreError};

    #[test]
    fn contextual_permission_denial_preserves_public_error_classification() {
        let core = CoreError::from(mixin_desktop_core::db::error::Error::PermissionDenied);
        let error = anyhow::Error::from(core).context("accept outgoing message");
        assert!(matches!(
            ClientError::from(error),
            ClientError::PermissionDenied
        ));
    }

    #[test]
    fn authorization_timeout_survives_anyhow_wrapping_as_public_timed_out() {
        let error = anyhow::Error::from(CoreError::TimedOut).context("wait for login");
        assert!(matches!(ClientError::from(error), ClientError::TimedOut));
    }

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
