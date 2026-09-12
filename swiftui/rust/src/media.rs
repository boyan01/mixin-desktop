use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures::{Stream, StreamExt as _};
use mixin_desktop_api::{AudioPlaybackEvent, MediaClient, VoiceRecorderEvent};
use tokio::sync::{Mutex, Notify};

#[derive(uniffi::Object)]
pub struct SwiftMediaHandle {
    client: Arc<MediaClient>,
}

#[derive(Clone, uniffi::Enum)]
pub enum MediaRecorderEvent {
    Changed { snapshot: VoiceRecorderSnapshot },
    Failed { message: String },
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

#[uniffi::export]
impl SwiftMediaHandle {
    pub fn subscribe_voice_recorder(&self) -> SwiftMediaRecorderSubscription {
        let events = self.client.subscribe_voice_recorder().map(Into::into);
        SwiftMediaRecorderSubscription {
            inner: CancellableStream::new(events),
        }
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

include!("generated/media.rs");
