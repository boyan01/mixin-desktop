use mixin_desktop_core::runtime::logging;

use crate::{ClientError, ClientResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

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

pub fn write_log(target: &str, level: LogLevel, message: &str) {
    let level = match level {
        LogLevel::Trace => log::Level::Trace,
        LogLevel::Debug => log::Level::Debug,
        LogLevel::Info => log::Level::Info,
        LogLevel::Warning => log::Level::Warn,
        LogLevel::Error => log::Level::Error,
    };
    log::log!(target: target, level, "{message}");
}
