use async_stream::stream;
use futures::Stream;
use mixin_desktop_media::{
    AudioPlaybackEvent, AudioPlaybackItem, AudioPlaybackSnapshot, AudioPlayer, VoiceRecorder,
    VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecording,
};

use crate::{ClientError, ClientResult};

#[derive(Clone, Default)]
pub struct MediaClient {
    player: AudioPlayer,
    recorder: VoiceRecorder,
}

impl MediaClient {
    pub fn new() -> Self {
        Self::default()
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
            loop {
                match receiver.recv().await {
                    Ok(event) => yield event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        yield VoiceRecorderEvent::Changed(recorder.snapshot());
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    pub async fn play_audio(
        &self,
        playlist: Vec<AudioPlaybackItem>,
        start_index: u64,
    ) -> ClientResult<()> {
        let player = self.player.clone();
        tokio::task::spawn_blocking(move || player.play(playlist, start_index))
            .await
            .map_err(join_error)??;
        Ok(())
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

    pub fn stop_audio(&self) {
        self.player.stop();
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
            loop {
                match receiver.recv().await {
                    Ok(event) => yield event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        yield AudioPlaybackEvent::Changed(player.snapshot());
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

fn join_error(error: tokio::task::JoinError) -> ClientError {
    ClientError::Internal(format!("media worker failed: {error}"))
}
