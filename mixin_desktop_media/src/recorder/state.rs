//! Recorder state shared by every target.
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceRecorderStatus {
    Idle,
    Recording,
    Recorded,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct VoiceRecording {
    pub session_id: u64,
    pub(super) _file: Option<Arc<RecordedFile>>,
    pub path: String,
    pub duration_millis: u64,
    pub waveform: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct RecordedFile(pub(super) PathBuf);

impl Drop for RecordedFile {
    fn drop(&mut self) {
        // A missing file needs no cleanup; drop cannot report other failures.
        let _ = std::fs::remove_file(&self.0);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceRecorderSnapshot {
    pub session_id: Option<u64>,
    pub status: VoiceRecorderStatus,
    pub recording: Option<VoiceRecording>,
    /// Captured PCM duration while recording; final encoded duration in preview.
    pub duration_millis: u64,
    /// Bounded recent capture peaks while recording; final protocol waveform in preview.
    pub waveform: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VoiceRecorderEvent {
    Changed(VoiceRecorderSnapshot),
    Failed(String),
}
