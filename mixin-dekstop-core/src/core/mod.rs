use std::error::Error;

pub use blaze::Blaze;

pub mod blaze;
pub(crate) mod crypto;
pub mod decrypt_message;
pub(crate) mod util;
mod model;

pub type AnyError = Box<dyn Error>;
