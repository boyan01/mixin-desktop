pub type Result<T, E = CoreError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not found")]
    NotFound,
    #[error("{message}")]
    Other { message: String },
}

impl From<anyhow::Error> for CoreError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other {
            message: error.to_string(),
        }
    }
}

impl From<crate::db::error::Error> for CoreError {
    fn from(error: crate::db::error::Error) -> Self {
        anyhow::Error::from(error).into()
    }
}

impl From<sdk::ApiError> for CoreError {
    fn from(error: sdk::ApiError) -> Self {
        anyhow::Error::from(error).into()
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(error: serde_json::Error) -> Self {
        anyhow::Error::from(error).into()
    }
}

#[cfg(test)]
mod tests {
    use super::CoreError;

    #[test]
    fn anyhow_error_converts_to_core_error_with_message() {
        let error = CoreError::from(anyhow::anyhow!("core failure"));

        assert_eq!(error.to_string(), "core failure");
    }

    #[test]
    fn not_found_has_stable_message() {
        assert_eq!(CoreError::NotFound.to_string(), "not found");
    }
}
