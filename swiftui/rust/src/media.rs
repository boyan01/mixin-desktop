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

#[derive(Clone, uniffi::Record)]
pub struct SwiftMediaAudioItem {
    pub id: String,
    pub path: String,
    pub duration_millis: u64,
}

#[derive(Clone, uniffi::Record)]
pub struct SwiftMediaVoiceRecording {
    pub path: String,
    pub duration_millis: u64,
    pub waveform: Vec<u8>,
}

#[derive(Clone, uniffi::Enum)]
pub enum SwiftMediaRecorderStatus {
    Idle,
    Recording,
    Recorded,
}

#[derive(Clone, uniffi::Record)]
pub struct SwiftMediaRecorderSnapshot {
    pub status: SwiftMediaRecorderStatus,
    pub recording: Option<SwiftMediaVoiceRecording>,
}

#[derive(Clone, uniffi::Enum)]
pub enum SwiftMediaRecorderEvent {
    Changed {
        snapshot: SwiftMediaRecorderSnapshot,
    },
    Failed {
        message: String,
    },
}

#[derive(Clone, uniffi::Enum)]
pub enum SwiftMediaPlaybackStatus {
    Idle,
    Playing,
    Paused,
}

#[derive(Clone, uniffi::Record)]
pub struct SwiftMediaPlaybackSnapshot {
    pub status: SwiftMediaPlaybackStatus,
    pub item: Option<SwiftMediaAudioItem>,
    pub position_millis: u64,
    pub duration_millis: u64,
    pub speed: f64,
}

#[derive(Clone, uniffi::Enum)]
pub enum SwiftMediaPlaybackEvent {
    Changed {
        snapshot: SwiftMediaPlaybackSnapshot,
    },
    Finished {
        id: String,
    },
    Failed {
        id: Option<String>,
        message: String,
    },
}

#[derive(uniffi::Object)]
pub struct SwiftMediaRecorderSubscription {
    inner: CancellableStream<SwiftMediaRecorderEvent>,
}

#[derive(uniffi::Object)]
pub struct SwiftMediaPlaybackSubscription {
    inner: CancellableStream<SwiftMediaPlaybackEvent>,
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

    pub async fn stop_voice_recording(&self) -> Result<SwiftMediaVoiceRecording, SwiftClientError> {
        Ok(self.client.stop_voice_recording().await?.into())
    }

    pub async fn cancel_voice_recording(&self) -> Result<(), SwiftClientError> {
        Ok(self.client.cancel_voice_recording().await?)
    }

    pub fn voice_recorder_snapshot(&self) -> SwiftMediaRecorderSnapshot {
        self.client.voice_recorder_snapshot().into()
    }

    pub fn subscribe_voice_recorder(&self) -> SwiftMediaRecorderSubscription {
        let events = self.client.subscribe_voice_recorder().map(Into::into);
        SwiftMediaRecorderSubscription {
            inner: CancellableStream::new(events),
        }
    }

    pub async fn play_audio(
        &self,
        playlist: Vec<SwiftMediaAudioItem>,
        start_index: u64,
    ) -> Result<(), SwiftClientError> {
        Ok(self
            .client
            .play_audio(playlist.into_iter().map(Into::into).collect(), start_index)
            .await?)
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

    pub fn audio_playback_snapshot(&self) -> SwiftMediaPlaybackSnapshot {
        self.client.audio_playback_snapshot().into()
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
    pub async fn next(&self) -> Option<SwiftMediaRecorderEvent> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SwiftMediaPlaybackSubscription {
    pub async fn next(&self) -> Option<SwiftMediaPlaybackEvent> {
        self.inner.next().await
    }

    pub fn cancel(&self) {
        self.inner.cancel();
    }
}

impl From<SwiftMediaAudioItem> for AudioPlaybackItem {
    fn from(value: SwiftMediaAudioItem) -> Self {
        Self {
            id: value.id,
            path: value.path,
            duration_millis: value.duration_millis,
        }
    }
}

impl From<AudioPlaybackItem> for SwiftMediaAudioItem {
    fn from(value: AudioPlaybackItem) -> Self {
        Self {
            id: value.id,
            path: value.path,
            duration_millis: value.duration_millis,
        }
    }
}

impl From<VoiceRecording> for SwiftMediaVoiceRecording {
    fn from(value: VoiceRecording) -> Self {
        Self {
            path: value.path,
            duration_millis: value.duration_millis,
            waveform: value.waveform,
        }
    }
}

impl From<VoiceRecorderStatus> for SwiftMediaRecorderStatus {
    fn from(value: VoiceRecorderStatus) -> Self {
        match value {
            VoiceRecorderStatus::Idle => Self::Idle,
            VoiceRecorderStatus::Recording => Self::Recording,
            VoiceRecorderStatus::Recorded => Self::Recorded,
        }
    }
}

impl From<VoiceRecorderSnapshot> for SwiftMediaRecorderSnapshot {
    fn from(value: VoiceRecorderSnapshot) -> Self {
        Self {
            status: value.status.into(),
            recording: value.recording.map(Into::into),
        }
    }
}

impl From<VoiceRecorderEvent> for SwiftMediaRecorderEvent {
    fn from(value: VoiceRecorderEvent) -> Self {
        match value {
            VoiceRecorderEvent::Changed(snapshot) => Self::Changed {
                snapshot: snapshot.into(),
            },
            VoiceRecorderEvent::Failed(message) => Self::Failed { message },
        }
    }
}

impl From<AudioPlaybackStatus> for SwiftMediaPlaybackStatus {
    fn from(value: AudioPlaybackStatus) -> Self {
        match value {
            AudioPlaybackStatus::Idle => Self::Idle,
            AudioPlaybackStatus::Playing => Self::Playing,
            AudioPlaybackStatus::Paused => Self::Paused,
        }
    }
}

impl From<AudioPlaybackSnapshot> for SwiftMediaPlaybackSnapshot {
    fn from(value: AudioPlaybackSnapshot) -> Self {
        Self {
            status: value.status.into(),
            item: value.item.map(Into::into),
            position_millis: value.position_millis,
            duration_millis: value.duration_millis,
            speed: value.speed,
        }
    }
}

impl From<AudioPlaybackEvent> for SwiftMediaPlaybackEvent {
    fn from(value: AudioPlaybackEvent) -> Self {
        match value {
            AudioPlaybackEvent::Changed(snapshot) => Self::Changed {
                snapshot: snapshot.into(),
            },
            AudioPlaybackEvent::Finished(id) => Self::Finished { id },
            AudioPlaybackEvent::Failed { id, message } => Self::Failed { id, message },
        }
    }
}
