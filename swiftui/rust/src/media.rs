use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures::{Stream, StreamExt as _};
use mixin_desktop_api::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus, MediaClient,
    VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus, VoiceRecording,
};
use tokio::sync::{Mutex, Notify};

use crate::error::SwiftClientError;

#[derive(uniffi::Object)]
pub struct SwiftMediaHandle {
    client: Arc<MediaClient>,
}

#[uniffi::remote(Record)]
pub struct AudioPlaybackItem {
    pub id: String,
    pub path: String,
    pub duration_millis: u64,
}

#[uniffi::remote(Record)]
pub struct VoiceRecording {
    pub path: String,
    pub duration_millis: u64,
    pub waveform: Vec<u8>,
}

#[uniffi::remote(Enum)]
pub enum VoiceRecorderStatus {
    Idle,
    Recording,
    Recorded,
}

#[uniffi::remote(Record)]
pub struct VoiceRecorderSnapshot {
    pub status: VoiceRecorderStatus,
    pub recording: Option<VoiceRecording>,
}

#[derive(Clone, uniffi::Enum)]
pub enum MediaRecorderEvent {
    Changed { snapshot: VoiceRecorderSnapshot },
    Failed { message: String },
}

#[uniffi::remote(Enum)]
pub enum AudioPlaybackStatus {
    Idle,
    Playing,
    Paused,
}

#[uniffi::remote(Record)]
pub struct AudioPlaybackSnapshot {
    pub status: AudioPlaybackStatus,
    pub item: Option<AudioPlaybackItem>,
    pub position_millis: u64,
    pub duration_millis: u64,
    pub speed: f64,
}

#[derive(Clone, uniffi::Enum)]
pub enum MediaPlaybackEvent {
    Changed { snapshot: AudioPlaybackSnapshot },
    Finished { id: String },
    Failed { id: Option<String>, message: String },
}

#[derive(uniffi::Object)]
pub struct SwiftMediaRecorderSubscription {
    inner: CancellableStream<MediaRecorderEvent>,
}

#[derive(uniffi::Object)]
pub struct SwiftMediaPlaybackSubscription {
    inner: CancellableStream<MediaPlaybackEvent>,
}

struct CancellableStream<T> {
    stream: Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>,
    cancelled: AtomicBool,
    cancel_notification: Notify,
}

impl<T> CancellableStream<T> {
    fn new(stream: impl Stream<Item = T> + Send + 'static) -> Self {
        Self {
            stream: Mutex::new(Box::pin(stream)),
            cancelled: AtomicBool::new(false),
            cancel_notification: Notify::new(),
        }
    }

    async fn next(&self) -> Option<T> {
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        let mut stream = self.stream.lock().await;
        tokio::select! {
            value = stream.next() => value,
            _ = self.cancel_notification.notified() => None,
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.cancel_notification.notify_one();
    }
}

impl SwiftMediaHandle {
    pub(crate) fn new(client: Arc<MediaClient>) -> Self {
        Self { client }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftMediaHandle {
    pub async fn start_voice_recording(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.start_voice_recording().await?)
    }

    pub async fn stop_voice_recording(&self) -> Result<VoiceRecording, SwiftClientError> {
        Ok(self.client.stop_voice_recording().await?)
    }

    pub async fn cancel_voice_recording(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.cancel_voice_recording().await?)
    }

    pub fn voice_recorder_snapshot(&self) -> VoiceRecorderSnapshot {
        self.client.voice_recorder_snapshot()
    }

    pub fn subscribe_voice_recorder(&self) -> SwiftMediaRecorderSubscription {
        let events = self.client.subscribe_voice_recorder().map(Into::into);
        SwiftMediaRecorderSubscription {
            inner: CancellableStream::new(events),
        }
    }

    pub async fn play_audio(
        &self,
        playlist: Vec<AudioPlaybackItem>,
        start_index: u64,
    ) -> Result<(), SwiftClientError> {
        Ok(self.client.play_audio(playlist, start_index).await?)
    }

    pub async fn pause_audio(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.pause_audio().await?)
    }

    pub async fn resume_audio(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.resume_audio().await?)
    }

    pub fn seek_audio(&self, position_millis: u64) {
        self.client.seek_audio(position_millis);
    }

    pub async fn set_audio_speed(&self, speed: f64) -> Result<(), SwiftClientError> {
        Ok(self.client.set_audio_speed(speed).await?)
    }

    pub fn stop_audio(&self) {
        self.client.stop_audio();
    }

    pub fn audio_playback_snapshot(&self) -> AudioPlaybackSnapshot {
        self.client.audio_playback_snapshot()
    }

    pub fn subscribe_audio_playback(&self) -> SwiftMediaPlaybackSubscription {
        let events = self.client.subscribe_audio_playback().map(Into::into);
        SwiftMediaPlaybackSubscription {
            inner: CancellableStream::new(events),
        }
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftMediaRecorderSubscription {
    pub async fn next(&self) -> Option<MediaRecorderEvent> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftMediaPlaybackSubscription {
    pub async fn next(&self) -> Option<MediaPlaybackEvent> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

impl From<VoiceRecorderEvent> for MediaRecorderEvent {
    fn from(value: VoiceRecorderEvent) -> Self {
        match value {
            VoiceRecorderEvent::Changed(snapshot) => Self::Changed { snapshot },
            VoiceRecorderEvent::Failed(message) => Self::Failed { message },
        }
    }
}

impl From<AudioPlaybackEvent> for MediaPlaybackEvent {
    fn from(value: AudioPlaybackEvent) -> Self {
        match value {
            AudioPlaybackEvent::Changed(snapshot) => Self::Changed { snapshot },
            AudioPlaybackEvent::Finished(id) => Self::Finished { id },
            AudioPlaybackEvent::Failed { id, message } => Self::Failed { id, message },
        }
    }
}
