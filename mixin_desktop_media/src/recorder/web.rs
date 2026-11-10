//! Browser builds have no audio input device: recording commands fail explicitly.
use std::future::Future;

use tokio::sync::broadcast;

use crate::{MediaError, MediaResult};

mod state;
pub use state::{VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus, VoiceRecording};

const UNSUPPORTED: MediaError = MediaError::Unsupported("voice recording");

#[derive(Clone)]
pub struct VoiceRecorder {
    events: broadcast::Sender<VoiceRecorderEvent>,
}

impl VoiceRecorder {
    pub fn new() -> Self {
        Self {
            events: broadcast::channel(1).0,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<VoiceRecorderEvent> {
        self.events.subscribe()
    }

    pub fn snapshot(&self) -> VoiceRecorderSnapshot {
        VoiceRecorderSnapshot {
            session_id: None,
            status: VoiceRecorderStatus::Idle,
            recording: None,
            duration_millis: 0,
            waveform: Vec::new(),
        }
    }

    pub fn is_closed(&self) -> bool {
        false
    }

    pub fn start(&self) -> MediaResult<()> {
        Err(UNSUPPORTED)
    }

    pub fn start_session(&self) -> MediaResult<u64> {
        Err(UNSUPPORTED)
    }

    pub fn close(&self) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        async { Ok(()) }
    }

    pub fn stop(&self) -> MediaResult<VoiceRecording> {
        Err(UNSUPPORTED)
    }

    pub fn stop_session(&self, _session_id: u64) -> MediaResult<VoiceRecording> {
        Err(UNSUPPORTED)
    }

    pub fn cancel_session(&self, _session_id: u64) -> MediaResult<()> {
        Ok(())
    }

    pub fn cancel(&self) -> MediaResult<()> {
        Ok(())
    }
}

impl Default for VoiceRecorder {
    fn default() -> Self {
        Self::new()
    }
}
