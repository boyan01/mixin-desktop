use mixin_desktop_api::{write_log, LogLevel};

#[uniffi::export]
pub fn log_swift(level: String, message: String) {
    let level = match level.as_str() {
        "verbose" => LogLevel::Trace,
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "warning" => LogLevel::Warning,
        "error" | "wtf" => LogLevel::Error,
        _ => LogLevel::Info,
    };
    write_log("swift", level, &message);
}
