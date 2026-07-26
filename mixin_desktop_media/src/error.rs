use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("no default {0} audio device is available")]
    MissingDevice(&'static str),
    #[error("audio device configuration failed: {0}")]
    DeviceConfiguration(String),
    #[error("audio stream failed: {0}")]
    Stream(String),
    #[error("voice recorder is already active")]
    RecorderActive,
    #[error("voice recorder is not active")]
    RecorderInactive,
    #[error("recorded audio is empty")]
    EmptyRecording,
    #[error("audio file is malformed: {0}")]
    MalformedAudio(String),
    #[error("audio time-stretch failed: {0}")]
    TimeStretch(String),
    #[error("Opus codec failed with code {0}")]
    Opus(i32),
    #[error("media I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub type MediaResult<T> = Result<T, MediaError>;
