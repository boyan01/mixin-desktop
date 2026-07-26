mod account;
mod desktop;
mod error;
mod login;
mod model;

pub use desktop::open_desktop;

uniffi::setup_scaffolding!();
