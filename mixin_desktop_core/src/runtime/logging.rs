//! Application logging.
//!
//! Native writes rolling log files through `log4rs`. wasm32 forwards records
//! to the browser console, where the developer tools provide the capture and
//! filtering surface instead of files.

use anyhow::Result;

use crate::db::path;

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::init;

pub fn directory() -> Result<String> {
    Ok(path::log_directory()?.to_string_lossy().into_owned())
}

/// wasm: install a console-backed logger. `set_logger` only fails when a
/// logger is already installed, which matches the native init's idempotence.
#[cfg(target_arch = "wasm32")]
pub fn init(_app_name: String, _app_version: String, _build_number: String) -> Result<()> {
    use log::{Level, Log, Metadata, Record};

    struct ConsoleLogger;

    impl Log for ConsoleLogger {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &Record<'_>) {
            let line = format!("{} {} - {}", record.level(), record.target(), record.args());
            let message = wasm_bindgen::JsValue::from_str(&line);
            match record.level() {
                Level::Error => web_sys::console::error_1(&message),
                Level::Warn => web_sys::console::warn_1(&message),
                Level::Info => web_sys::console::info_1(&message),
                _ => web_sys::console::log_1(&message),
            }
        }

        fn flush(&self) {}
    }

    static CONSOLE_LOGGER: ConsoleLogger = ConsoleLogger;
    if log::set_logger(&CONSOLE_LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
    Ok(())
}
