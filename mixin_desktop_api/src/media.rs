use std::future::Future;

use async_stream::stream;
use futures::Stream;
use mixin_desktop_media::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlaybackStatus, AudioPlayer,
    VoiceRecorder, VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecording,
};

use crate::{ClientError, ClientResult};

/// An account-loaded Lottie composition with bounded shared software rendering.
#[derive(Clone)]
pub struct StickerAnimationClient(pub(crate) mixin_desktop_media::sticker::StickerAnimation);

impl StickerAnimationClient {
    pub fn first_frame(&self) -> std::sync::Arc<str> {
        self.0.first_frame.clone()
    }

    pub async fn frame(&self, elapsed_millis: u64) -> ClientResult<std::sync::Arc<str>> {
        Ok(self.0.frame(elapsed_millis).await?)
    }
}

#[derive(Clone, Default)]
pub struct MediaClient {
    player: AudioPlayer,
    recorder: VoiceRecorder,
}

impl MediaClient {
    /// Decodes an inline image or BlurHash placeholder within shared resource limits.
    pub async fn thumbnail_image(&self, data: String) -> ClientResult<Option<String>> {
        Ok(mixin_desktop_media::thumbnail::image(data).await?)
    }

    pub async fn prepare_clipboard_file(
        &self,
        name: String,
        bytes: Vec<u8>,
        media: bool,
        compress: bool,
    ) -> ClientResult<mixin_desktop_media::PreparedAttachment> {
        Ok(
            mixin_desktop_media::attachment::prepare_clipboard_file(name, bytes, media, compress)
                .await?,
        )
    }

    /// Normalizes a bounded static avatar source and retains its temporary file.
    pub async fn prepare_avatar(
        &self,
        path: String,
    ) -> ClientResult<mixin_desktop_media::PreparedAttachment> {
        Ok(mixin_desktop_media::avatar::prepare(path).await?)
    }

    /// Exports a square crop up to 512 pixels. Group uploads use JPEG/base64url;
    /// profile uploads use PNG/standard base64.
    pub async fn encode_avatar(
        &self,
        source: mixin_desktop_media::PreparedAttachment,
        crop: mixin_desktop_media::ImageEdit,
        group: bool,
    ) -> ClientResult<String> {
        Ok(mixin_desktop_media::avatar::encode(source, crop, group).await?)
    }

    pub async fn edit_image(
        &self,
        path: String,
        edits: Vec<mixin_desktop_media::ImageEdit>,
    ) -> ClientResult<mixin_desktop_media::PreparedAttachment> {
        Ok(mixin_desktop_media::image_edit::apply(path, edits).await?)
    }

    pub async fn prepare_archive(
        &self,
        items: Vec<mixin_desktop_media::PreparedAttachment>,
        password: Option<String>,
    ) -> ClientResult<mixin_desktop_media::PreparedAttachment> {
        Ok(mixin_desktop_media::archive::prepare(items, password).await?)
    }

    pub async fn prepare_attachment(
        &self,
        path: String,
        media: bool,
        compress: bool,
    ) -> ClientResult<mixin_desktop_media::PreparedAttachment> {
        Ok(mixin_desktop_media::attachment::prepare(path, media, compress).await?)
    }

    pub async fn prepare_clipboard_image(
        &self,
        data_uri: String,
        media: bool,
        compress: bool,
    ) -> ClientResult<mixin_desktop_media::PreparedAttachment> {
        Ok(
            mixin_desktop_media::attachment::prepare_clipboard_image(data_uri, media, compress)
                .await?,
        )
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start_voice_session(&self) -> ClientResult<u64> {
        let recorder = self.recorder.clone();
        Ok(
            tokio::task::spawn_blocking(move || recorder.start_session())
                .await
                .map_err(join_error)??,
        )
    }

    pub async fn stop_voice_session(&self, session_id: u64) -> ClientResult<VoiceRecording> {
        let recorder = self.recorder.clone();
        Ok(
            tokio::task::spawn_blocking(move || recorder.stop_session(session_id))
                .await
                .map_err(join_error)??,
        )
    }

    pub async fn cancel_voice_session(&self, session_id: u64) -> ClientResult<()> {
        let recorder = self.recorder.clone();
        Ok(
            tokio::task::spawn_blocking(move || recorder.cancel_session(session_id))
                .await
                .map_err(join_error)??,
        )
    }

    pub async fn start_voice_recording(&self) -> ClientResult<()> {
        let recorder = self.recorder.clone();
        tokio::task::spawn_blocking(move || recorder.start())
            .await
            .map_err(join_error)??;
        Ok(())
    }

    pub async fn stop_voice_recording(&self) -> ClientResult<VoiceRecording> {
        let recorder = self.recorder.clone();
        Ok(tokio::task::spawn_blocking(move || recorder.stop())
            .await
            .map_err(join_error)??)
    }

    pub async fn cancel_voice_recording(&self) -> ClientResult<()> {
        let recorder = self.recorder.clone();
        tokio::task::spawn_blocking(move || recorder.cancel())
            .await
            .map_err(join_error)??;
        Ok(())
    }

    pub fn voice_recorder_snapshot(&self) -> VoiceRecorderSnapshot {
        self.recorder.snapshot()
    }

    pub fn subscribe_voice_recorder(
        &self,
    ) -> impl Stream<Item = VoiceRecorderEvent> + Send + 'static {
        let mut receiver = self.recorder.subscribe();
        let recorder = self.recorder.clone();
        stream! {
            let mut snapshot = recorder.snapshot();
            yield VoiceRecorderEvent::Changed(snapshot.clone());
            let mut ticks = tokio::time::interval(std::time::Duration::from_millis(100));
            ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                if recorder.is_closed() {
                    break;
                }
                tokio::select! {
                    result = receiver.recv() => match result {
                        Ok(VoiceRecorderEvent::Changed(value)) => {
                            snapshot = value.clone();
                            yield VoiceRecorderEvent::Changed(value);
                        }
                        Ok(event) => yield event,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            snapshot = recorder.snapshot();
                            yield VoiceRecorderEvent::Changed(snapshot.clone());
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                    _ = ticks.tick(), if snapshot.status == crate::VoiceRecorderStatus::Recording => {
                        let current = recorder.snapshot();
                        if current != snapshot {
                            snapshot = current.clone();
                            yield VoiceRecorderEvent::Changed(current);
                        }
                    }
                }
            }
        }
    }

    /// Permanently closes this account's media session before cleanup is polled.
    pub fn close(&self) -> impl Future<Output = ClientResult<()>> + Send + 'static {
        let playback = self.player.close();
        let recording = self.recorder.close();
        async move {
            let (playback, recording) = tokio::join!(playback, recording);
            playback?;
            recording?;
            Ok(())
        }
    }

    pub fn play_audio(
        &self,
        playlist: Vec<AudioPlaybackItem>,
        start_index: u64,
    ) -> impl Future<Output = ClientResult<()>> + Send + 'static {
        let playback = self.player.play_async(playlist, start_index);
        async move { Ok(playback.await?) }
    }

    pub async fn pause_audio(&self) -> ClientResult<()> {
        let player = self.player.clone();
        tokio::task::spawn_blocking(move || player.pause())
            .await
            .map_err(join_error)??;
        Ok(())
    }

    pub async fn resume_audio(&self) -> ClientResult<()> {
        let player = self.player.clone();
        tokio::task::spawn_blocking(move || player.resume())
            .await
            .map_err(join_error)??;
        Ok(())
    }

    pub fn seek_audio(&self, position_millis: u64) {
        self.player.seek(position_millis);
    }

    pub async fn set_audio_speed(&self, speed: f64) -> ClientResult<()> {
        let player = self.player.clone();
        tokio::task::spawn_blocking(move || player.set_speed(speed))
            .await
            .map_err(join_error)??;
        Ok(())
    }

    pub fn stop_audio(&self) -> impl Future<Output = ClientResult<()>> + Send + 'static {
        let stopping = self.player.stop_async();
        async move { Ok(stopping.await?) }
    }

    pub fn audio_playback_snapshot(&self) -> AudioPlaybackSnapshot {
        self.player.snapshot()
    }

    pub fn subscribe_audio_playback(
        &self,
    ) -> impl Stream<Item = AudioPlaybackEvent> + Send + 'static {
        let mut receiver = self.player.subscribe();
        let player = self.player.clone();
        stream! {
            let mut previous = player.snapshot();
            // Playback position follows the audio clock even when no control
            // state changes. This timer is disposed with the subscription.
            let mut clock = tokio::time::interval(std::time::Duration::from_millis(200));
            clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    event = receiver.recv() => match event {
                        Ok(event) => {
                            if let AudioPlaybackEvent::Changed(snapshot) = &event {
                                previous = snapshot.clone();
                                clock.reset();
                            }
                            yield event;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            previous = player.snapshot();
                            yield AudioPlaybackEvent::Changed(previous.clone());
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                    _ = clock.tick(), if previous.status == AudioPlaybackStatus::Playing => {
                        let next = player.snapshot();
                        if next != previous {
                            previous = next.clone();
                            yield AudioPlaybackEvent::Changed(next);
                        }
                    }
                }
            }
        }
    }
}

fn join_error(error: tokio::task::JoinError) -> ClientError {
    ClientError::Internal(format!("media worker failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn closing_media_ends_existing_and_late_recorder_subscriptions() {
        let media = MediaClient::default();
        let mut existing = Box::pin(media.subscribe_voice_recorder());
        assert!(matches!(existing.next().await,
            Some(VoiceRecorderEvent::Changed(snapshot))
                if snapshot.status == crate::VoiceRecorderStatus::Idle));
        media.close().await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(2), existing.next())
                .await
                .unwrap()
                .is_none()
        );
        let mut late = Box::pin(media.subscribe_voice_recorder());
        assert!(matches!(late.next().await,
            Some(VoiceRecorderEvent::Changed(snapshot))
                if snapshot.status == crate::VoiceRecorderStatus::Idle
                    && snapshot.duration_millis == 0 && snapshot.waveform.is_empty()));
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(2), late.next())
                .await
                .unwrap()
                .is_none()
        );
    }
}
