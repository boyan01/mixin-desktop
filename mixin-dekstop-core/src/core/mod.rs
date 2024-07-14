use std::error::Error;

pub use blaze::Blaze;

pub mod blaze;
pub mod decrypt_message;
pub(crate) mod util;

pub type AnyError = Box<dyn Error>;