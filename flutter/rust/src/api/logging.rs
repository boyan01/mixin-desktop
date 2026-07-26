use crate::{CoreError, Result};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() -> Result<(), CoreError> {
    flutter_rust_bridge::setup_backtrace();
    Ok(())
}

pub fn init(app_name: String, app_version: String, build_number: String) -> Result<(), CoreError> {
    Ok(mixin_desktop_api::init_logging(
        app_name,
        app_version,
        build_number,
    )?)
}

pub fn directory() -> Result<String, CoreError> {
    Ok(mixin_desktop_api::log_directory()?)
}

#[flutter_rust_bridge::frb(sync)]
pub fn log_flutter(level: String, message: String) {
    match level.as_str() {
        "verbose" => log::trace!(target: "flutter", "{message}"),
        "debug" => log::debug!(target: "flutter", "{message}"),
        "info" => log::info!(target: "flutter", "{message}"),
        "warning" => log::warn!(target: "flutter", "{message}"),
        "error" | "wtf" => log::error!(target: "flutter", "{message}"),
        _ => log::info!(target: "flutter", "{message}"),
    }
}
