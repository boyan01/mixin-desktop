use mixin_desktop_core::runtime::logging;

use crate::{ClientError, ClientResult};

pub fn init_logging(
    app_name: String,
    app_version: String,
    build_number: String,
) -> ClientResult<()> {
    logging::init(app_name, app_version, build_number)
        .map_err(|error| ClientError::Internal(error.to_string()))
}

pub fn log_directory() -> ClientResult<String> {
    logging::directory().map_err(Into::into)
}
