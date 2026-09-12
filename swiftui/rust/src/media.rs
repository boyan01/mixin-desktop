use std::sync::Arc;

use crate::subscription::{subscription, CancellableStream};
use futures::StreamExt as _;
use mixin_desktop_api::{AudioPlaybackEvent, MediaClient, VoiceRecorderEvent};

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

subscription!(SwiftMediaRecorderSubscription, MediaRecorderEvent);

subscription!(SwiftMediaPlaybackSubscription, MediaPlaybackEvent);

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
