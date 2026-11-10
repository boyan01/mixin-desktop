//! Browser builds have no audio output device: playback commands fail explicitly.
use std::future::Future;

use tokio::sync::broadcast;

use crate::{MediaError, MediaResult};

mod state;
pub use state::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus,
};

const UNSUPPORTED: MediaError = MediaError::Unsupported("audio playback");

#[derive(Clone)]
pub struct AudioPlayer {
    events: broadcast::Sender<AudioPlaybackEvent>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            events: broadcast::channel(1).0,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AudioPlaybackEvent> {
        self.events.subscribe()
    }

    pub fn play_async(
        &self,
        _playlist: Vec<AudioPlaybackItem>,
        _start_index: u64,
    ) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        async { Err(UNSUPPORTED) }
    }

    pub fn close(&self) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        async { Ok(()) }
    }

    pub fn stop_async(&self) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        async { Ok(()) }
    }

    pub fn pause(&self) -> MediaResult<()> {
        Err(UNSUPPORTED)
    }

    pub fn resume(&self) -> MediaResult<()> {
        Err(UNSUPPORTED)
    }

    pub fn seek(&self, _position_millis: u64) {}

    pub fn set_speed(&self, _speed: f64) -> MediaResult<()> {
        Err(UNSUPPORTED)
    }

    pub fn snapshot(&self) -> AudioPlaybackSnapshot {
        AudioPlaybackSnapshot {
            status: AudioPlaybackStatus::Idle,
            item: None,
            position_millis: 0,
            duration_millis: 0,
            speed: 1.0,
        }
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}
