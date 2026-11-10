pub mod archive;
pub mod attachment;
pub mod avatar;
pub use attachment::PreparedAttachment;
pub mod image_edit;
pub use image_edit::ImageEdit;
#[cfg(not(target_arch = "wasm32"))]
mod codec;
mod error;
#[cfg_attr(target_arch = "wasm32", path = "player/web.rs")]
mod player;
#[cfg_attr(target_arch = "wasm32", path = "recorder/web.rs")]
mod recorder;
pub mod sticker;
pub mod thumbnail;

pub use error::{MediaError, MediaResult};
pub use player::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus, AudioPlayer,
};
pub use recorder::{
    VoiceRecorder, VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus, VoiceRecording,
};

/// Runs CPU-bound or blocking media work: on Tokio's blocking pool on the
/// desktop, and inline in the browser, whose single thread has no Tokio runtime.
#[cfg(not(target_arch = "wasm32"))]
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, tokio::task::JoinError> {
    tokio::task::spawn_blocking(work).await
}

/// Runs CPU-bound or blocking media work: on Tokio's blocking pool on the
/// desktop, and inline in the browser, whose single thread has no Tokio runtime.
#[cfg(target_arch = "wasm32")]
pub async fn blocking<T>(work: impl FnOnce() -> T) -> Result<T, std::convert::Infallible> {
    Ok(work())
}
