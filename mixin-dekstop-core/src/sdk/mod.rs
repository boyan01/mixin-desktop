pub mod client;
pub mod err;
mod credential;

pub use err::{Error, ApiError};
pub use credential::{Credential, KeyStore};