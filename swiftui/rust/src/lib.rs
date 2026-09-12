mod account;
mod desktop;
mod error;
mod logging;
mod login;
mod media;
mod model;
mod subscription;

pub use desktop::open_desktop;

uniffi::setup_scaffolding!();
