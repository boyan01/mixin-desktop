#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use futures::StreamExt as _;
#[cfg(not(target_arch = "wasm32"))]
use mixin_desktop_api::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus, MediaClient,
    VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus, VoiceRecording,
};

use crate::{frb_generated::StreamSink, CoreError, Result};

#[flutter_rust_bridge::frb(opaque)]
pub struct MediaHandle {
    #[cfg(not(target_arch = "wasm32"))]
    client: Arc<MediaClient>,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct MediaAudioItem {
    pub id: String,
    pub path: String,
    pub duration_millis: u64,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct MediaVoiceRecording {
    pub path: String,
    pub duration_millis: u64,
    pub waveform: Vec<u8>,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum MediaRecorderStatus {
    Idle,
    Recording,
    Recorded,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct MediaRecorderSnapshot {
    pub status: MediaRecorderStatus,
    pub recording: Option<MediaVoiceRecording>,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum MediaRecorderEvent {
    Changed { snapshot: MediaRecorderSnapshot },
    Failed { message: String },
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum MediaPlaybackStatus {
    Idle,
    Playing,
    Paused,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub struct MediaPlaybackSnapshot {
    pub status: MediaPlaybackStatus,
    pub item: Option<MediaAudioItem>,
    pub position_millis: u64,
    pub duration_millis: u64,
    pub speed: f64,
}

#[derive(Clone)]
#[flutter_rust_bridge::frb(non_opaque)]
pub enum MediaPlaybackEvent {
    Changed { snapshot: MediaPlaybackSnapshot },
    Finished { id: String },
    Failed { id: Option<String>, message: String },
}

impl MediaHandle {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new(client: Arc<MediaClient>) -> Self {
        Self { client }
    }

    pub async fn start_voice_recording(&self) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.start_voice_recording().await?)
        }
    }

    pub async fn stop_voice_recording(&self) -> Result<MediaVoiceRecording, CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.stop_voice_recording().await?.into())
        }
    }

    pub async fn cancel_voice_recording(&self) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.cancel_voice_recording().await?)
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn voice_recorder_snapshot(&self) -> Result<MediaRecorderSnapshot, CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.voice_recorder_snapshot().into())
        }
    }

    pub async fn voice_recorder_events(
        &self,
        sink: StreamSink<MediaRecorderEvent>,
    ) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (sink,);
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let events = self.client.subscribe_voice_recorder();
            futures::pin_mut!(events);
            while let Some(event) = events.next().await {
                if sink.add(event.into()).is_err() {
                    break;
                }
            }
            Ok(())
        }
    }

    pub async fn play_audio(
        &self,
        playlist: Vec<MediaAudioItem>,
        start_index: u64,
    ) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (playlist, start_index);
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self
                .client
                .play_audio(playlist.into_iter().map(Into::into).collect(), start_index)
                .await?)
        }
    }

    pub async fn pause_audio(&self) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.pause_audio().await?)
        }
    }

    pub async fn resume_audio(&self) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.resume_audio().await?)
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn seek_audio(&self, position_millis: u64) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (position_millis,);
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.client.seek_audio(position_millis);
            Ok(())
        }
    }

    pub async fn set_audio_speed(&self, speed: f64) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (speed,);
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.set_audio_speed(speed).await?)
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn stop_audio(&self) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.client.stop_audio();
            Ok(())
        }
    }

    #[flutter_rust_bridge::frb(sync)]
    pub fn audio_playback_snapshot(&self) -> Result<MediaPlaybackSnapshot, CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(self.client.audio_playback_snapshot().into())
        }
    }

    pub async fn audio_playback_events(
        &self,
        sink: StreamSink<MediaPlaybackEvent>,
    ) -> Result<(), CoreError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (sink,);
            Err(CoreError::Other {
                message: "Native media is not supported in the browser".into(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let events = self.client.subscribe_audio_playback();
            futures::pin_mut!(events);
            while let Some(event) = events.next().await {
                if sink.add(event.into()).is_err() {
                    break;
                }
            }
            Ok(())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<MediaAudioItem> for AudioPlaybackItem {
    fn from(value: MediaAudioItem) -> Self {
        Self {
            id: value.id,
            path: value.path,
            duration_millis: value.duration_millis,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<AudioPlaybackItem> for MediaAudioItem {
    fn from(value: AudioPlaybackItem) -> Self {
        Self {
            id: value.id,
            path: value.path,
            duration_millis: value.duration_millis,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<VoiceRecording> for MediaVoiceRecording {
    fn from(value: VoiceRecording) -> Self {
        Self {
            path: value.path,
            duration_millis: value.duration_millis,
            waveform: value.waveform,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<VoiceRecorderStatus> for MediaRecorderStatus {
    fn from(value: VoiceRecorderStatus) -> Self {
        match value {
            VoiceRecorderStatus::Idle => Self::Idle,
            VoiceRecorderStatus::Recording => Self::Recording,
            VoiceRecorderStatus::Recorded => Self::Recorded,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<VoiceRecorderSnapshot> for MediaRecorderSnapshot {
    fn from(value: VoiceRecorderSnapshot) -> Self {
        Self {
            status: value.status.into(),
            recording: value.recording.map(Into::into),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<VoiceRecorderEvent> for MediaRecorderEvent {
    fn from(value: VoiceRecorderEvent) -> Self {
        match value {
            VoiceRecorderEvent::Changed(snapshot) => Self::Changed {
                snapshot: snapshot.into(),
            },
            VoiceRecorderEvent::Failed(message) => Self::Failed { message },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<AudioPlaybackStatus> for MediaPlaybackStatus {
    fn from(value: AudioPlaybackStatus) -> Self {
        match value {
            AudioPlaybackStatus::Idle => Self::Idle,
            AudioPlaybackStatus::Playing => Self::Playing,
            AudioPlaybackStatus::Paused => Self::Paused,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<AudioPlaybackSnapshot> for MediaPlaybackSnapshot {
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

#[cfg(not(target_arch = "wasm32"))]
impl From<AudioPlaybackEvent> for MediaPlaybackEvent {
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
