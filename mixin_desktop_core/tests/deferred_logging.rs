//! The native host may attempt logger installation between discovery and launch.
use mixin_desktop_core::{db::path, runtime::logging};

struct HostLogger;
impl log::Log for HostLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, _: &log::Record<'_>) {}
    fn flush(&self) {}
}

#[test]
fn discovery_reserves_logging_without_touching_data_then_launch_writes_records() {
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("data");
    path::configure_data_directory(data.clone()).unwrap();
    logging::install_deferred().unwrap();
    assert!(!data.exists(), "discovery must not open application data");
    assert!(
        log::set_logger(&HostLogger).is_err(),
        "the host must retain application logging"
    );
    logging::init("Mixin".into(), "test".into(), "1".into()).unwrap();
    logging::init("Mixin".into(), "test".into(), "1".into()).unwrap();
    log::info!("deferred-logger-info-regression");
    log::warn!("deferred-logger-warning-regression");
    log::logger().flush();
    let text = std::fs::read_to_string(data.join("log/log_0.log")).unwrap();
    assert!(text.contains("deferred-logger-info-regression"));
    assert!(text.contains("deferred-logger-warning-regression"));
    assert_eq!(text.matches("# app: Mixin").count(), 1);
}
