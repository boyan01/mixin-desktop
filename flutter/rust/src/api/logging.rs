use anyhow::Result;

#[flutter_rust_bridge::frb(init)]
pub fn init_app() -> Result<()> {
    flutter_rust_bridge::setup_backtrace();
    Ok(())
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
