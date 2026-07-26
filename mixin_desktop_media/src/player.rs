use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use tokio::sync::broadcast;

use crate::codec::{decode_ogg_opus, DecodedAudio};
use crate::{MediaError, MediaResult};

#[derive(Clone, Debug, PartialEq)]
pub struct AudioPlaybackItem {
    pub id: String,
    pub path: String,
    pub duration_millis: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioPlaybackStatus {
    Idle,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioPlaybackSnapshot {
    pub status: AudioPlaybackStatus,
    pub item: Option<AudioPlaybackItem>,
    pub position_millis: u64,
    pub duration_millis: u64,
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioPlaybackEvent {
    Changed(AudioPlaybackSnapshot),
    Finished(String),
    Failed { id: Option<String>, message: String },
}

#[derive(Clone)]
pub struct AudioPlayer {
    inner: Arc<PlayerInner>,
}

struct PlayerInner {
    state: Mutex<PlayerState>,
    events: broadcast::Sender<AudioPlaybackEvent>,
}

struct PlayerState {
    status: AudioPlaybackStatus,
    playlist: Vec<AudioPlaybackItem>,
    index: usize,
    stream: Option<Stream>,
    cursor: Option<Arc<PlaybackCursor>>,
    speed: f64,
}

struct PlaybackCursor {
    audio: DecodedAudio,
    buffer: Mutex<PlaybackBuffer>,
    ended: AtomicBool,
    advance_scheduled: AtomicBool,
}

struct PlaybackBuffer {
    samples: Vec<f32>,
    frame: f64,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        Self {
            inner: Arc::new(PlayerInner {
                state: Mutex::new(PlayerState {
                    status: AudioPlaybackStatus::Idle,
                    playlist: Vec::new(),
                    index: 0,
                    stream: None,
                    cursor: None,
                    speed: 1.0,
                }),
                events,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AudioPlaybackEvent> {
        self.inner.events.subscribe()
    }

    pub fn play(&self, playlist: Vec<AudioPlaybackItem>, start_index: u64) -> MediaResult<()> {
        if playlist.is_empty() || start_index as usize >= playlist.len() {
            return Err(MediaError::MalformedAudio(
                "playback list or start index is invalid".to_string(),
            ));
        }
        {
            let mut state = self.inner.state.lock().expect("player state poisoned");
            state.playlist = playlist;
            state.index = start_index as usize;
        }
        start_current(&self.inner)
    }

    pub fn pause(&self) -> MediaResult<()> {
        let mut state = self.inner.state.lock().expect("player state poisoned");
        if let Some(stream) = &state.stream {
            stream
                .pause()
                .map_err(|error| MediaError::Stream(error.to_string()))?;
            state.status = AudioPlaybackStatus::Paused;
        }
        drop(state);
        emit_snapshot(&self.inner);
        Ok(())
    }

    pub fn resume(&self) -> MediaResult<()> {
        let mut state = self.inner.state.lock().expect("player state poisoned");
        if let Some(stream) = &state.stream {
            stream
                .play()
                .map_err(|error| MediaError::Stream(error.to_string()))?;
            state.status = AudioPlaybackStatus::Playing;
        }
        drop(state);
        emit_snapshot(&self.inner);
        Ok(())
    }

    pub fn seek(&self, position_millis: u64) {
        let state = self.inner.state.lock().expect("player state poisoned");
        let Some(cursor) = &state.cursor else {
            return;
        };
        let frame = position_millis as f64 * cursor.audio.sample_rate as f64 / 1_000.0;
        let original_frames = cursor.audio.samples.len() / usize::from(cursor.audio.channels);
        let mut buffer = cursor.buffer.lock().expect("player cursor poisoned");
        let rendered_frames = buffer.samples.len() / usize::from(cursor.audio.channels);
        buffer.frame = if original_frames == 0 {
            0.0
        } else {
            (frame / original_frames as f64 * rendered_frames as f64)
                .clamp(0.0, rendered_frames as f64)
        };
        cursor.ended.store(false, Ordering::Release);
        cursor.advance_scheduled.store(false, Ordering::Release);
        drop(buffer);
        drop(state);
        emit_snapshot(&self.inner);
    }

    pub fn set_speed(&self, speed: f64) -> MediaResult<()> {
        if !(0.5..=2.0).contains(&speed) {
            return Err(MediaError::DeviceConfiguration(format!(
                "playback speed {speed} is outside 0.5...2.0"
            )));
        }
        let mut state = self.inner.state.lock().expect("player state poisoned");
        if (state.speed - speed).abs() < f64::EPSILON {
            return Ok(());
        }
        let was_playing = state.status == AudioPlaybackStatus::Playing;
        if was_playing {
            if let Some(stream) = &state.stream {
                stream
                    .pause()
                    .map_err(|error| MediaError::Stream(error.to_string()))?;
            }
        }
        if let Some(cursor) = &state.cursor {
            let samples = match stretch_audio(&cursor.audio, speed) {
                Ok(samples) => samples,
                Err(error) => {
                    if was_playing {
                        if let Some(stream) = &state.stream {
                            let _ = stream.play();
                        }
                    }
                    return Err(error);
                }
            };
            let mut buffer = cursor.buffer.lock().expect("player cursor poisoned");
            let channels = usize::from(cursor.audio.channels);
            let old_frames = buffer.samples.len() / channels;
            let progress = if old_frames == 0 {
                0.0
            } else {
                buffer.frame / old_frames as f64
            };
            let new_frames = samples.len() / channels;
            buffer.samples = samples;
            buffer.frame = (progress * new_frames as f64).clamp(0.0, new_frames as f64);
            cursor.ended.store(false, Ordering::Release);
            cursor.advance_scheduled.store(false, Ordering::Release);
        }
        state.speed = speed;
        if was_playing {
            if let Some(stream) = &state.stream {
                stream
                    .play()
                    .map_err(|error| MediaError::Stream(error.to_string()))?;
            }
        }
        drop(state);
        emit_snapshot(&self.inner);
        Ok(())
    }

    pub fn stop(&self) {
        stop_inner(&self.inner, true);
    }

    pub fn snapshot(&self) -> AudioPlaybackSnapshot {
        snapshot_for(&self.inner.state.lock().expect("player state poisoned"))
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

fn start_current(inner: &Arc<PlayerInner>) -> MediaResult<()> {
    let (item, speed) = {
        let state = inner.state.lock().expect("player state poisoned");
        (state.playlist[state.index].clone(), state.speed)
    };
    let bytes = fs::read(&item.path)?;
    let audio = decode_ogg_opus(bytes)?;
    let samples = stretch_audio(&audio, speed)?;
    let cursor = Arc::new(PlaybackCursor {
        audio,
        buffer: Mutex::new(PlaybackBuffer {
            samples,
            frame: 0.0,
        }),
        ended: AtomicBool::new(false),
        advance_scheduled: AtomicBool::new(false),
    });
    let stream = build_output_stream(cursor.clone(), Arc::downgrade(inner), item.id.clone())?;
    stream
        .play()
        .map_err(|error| MediaError::Stream(error.to_string()))?;
    {
        let mut state = inner.state.lock().expect("player state poisoned");
        state.stream = Some(stream);
        state.cursor = Some(cursor);
        state.status = AudioPlaybackStatus::Playing;
    }
    emit_snapshot(inner);
    Ok(())
}

fn build_output_stream(
    cursor: Arc<PlaybackCursor>,
    inner: Weak<PlayerInner>,
    item_id: String,
) -> MediaResult<Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(MediaError::MissingDevice("output"))?;
    let supported = device
        .default_output_config()
        .map_err(|error| MediaError::DeviceConfiguration(error.to_string()))?;
    let config: StreamConfig = supported.into();
    let output_channels = usize::from(config.channels);
    let output_rate = config.sample_rate;
    macro_rules! build {
        ($sample:ty) => {
            build_typed_output_stream::<$sample>(
                &device,
                config,
                output_channels,
                output_rate,
                cursor,
                inner,
                item_id,
            )
        };
    }
    match supported.sample_format() {
        SampleFormat::F32 => build!(f32),
        SampleFormat::F64 => build!(f64),
        SampleFormat::I8 => build!(i8),
        SampleFormat::I16 => build!(i16),
        SampleFormat::I32 => build!(i32),
        SampleFormat::I64 => build!(i64),
        SampleFormat::U8 => build!(u8),
        SampleFormat::U16 => build!(u16),
        SampleFormat::U32 => build!(u32),
        SampleFormat::U64 => build!(u64),
        format => Err(MediaError::DeviceConfiguration(format!(
            "unsupported output sample format {format}"
        ))),
    }
}

fn build_typed_output_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    output_channels: usize,
    output_rate: u32,
    cursor: Arc<PlaybackCursor>,
    inner: Weak<PlayerInner>,
    item_id: String,
) -> MediaResult<Stream>
where
    T: Sample + SizedSample + FromSample<f32>,
{
    let error_inner = inner.clone();
    let error_item_id = item_id.clone();
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                fill_output(output, output_channels, output_rate, &cursor);
                if cursor.ended.load(Ordering::Acquire)
                    && !cursor.advance_scheduled.swap(true, Ordering::AcqRel)
                {
                    let inner = inner.clone();
                    let item_id = item_id.clone();
                    std::thread::spawn(move || advance(inner, item_id));
                }
            },
            move |error| {
                if let Some(inner) = error_inner.upgrade() {
                    let _ = inner.events.send(AudioPlaybackEvent::Failed {
                        id: Some(error_item_id.clone()),
                        message: error.to_string(),
                    });
                }
            },
            None,
        )
        .map_err(|error| MediaError::Stream(error.to_string()))
}

fn fill_output<T>(
    output: &mut [T],
    output_channels: usize,
    output_rate: u32,
    cursor: &PlaybackCursor,
) where
    T: Sample + FromSample<f32>,
{
    let source_channels = usize::from(cursor.audio.channels);
    let step = cursor.audio.sample_rate as f64 / output_rate as f64;
    let mut buffer = cursor.buffer.lock().expect("player cursor poisoned");
    let source_frames = buffer.samples.len() / source_channels;
    for frame in output.chunks_mut(output_channels) {
        if buffer.frame >= source_frames as f64 {
            for sample in frame {
                *sample = T::from_sample(0.0);
            }
            if !cursor.ended.swap(true, Ordering::AcqRel) {
                continue;
            }
            continue;
        }
        let lower = buffer.frame.floor() as usize;
        let upper = (lower + 1).min(source_frames - 1);
        let fraction = (buffer.frame - lower as f64) as f32;
        for (channel, output_sample) in frame.iter_mut().enumerate() {
            let source_channel = channel.min(source_channels - 1);
            let first = buffer.samples[lower * source_channels + source_channel];
            let second = buffer.samples[upper * source_channels + source_channel];
            *output_sample = T::from_sample(first + (second - first) * fraction);
        }
        buffer.frame += step;
    }
}

fn advance(inner: Weak<PlayerInner>, item_id: String) {
    let Some(inner) = inner.upgrade() else {
        return;
    };
    let has_next = {
        let mut state = inner.state.lock().expect("player state poisoned");
        let Some(current) = state.playlist.get(state.index) else {
            return;
        };
        if current.id != item_id {
            return;
        }
        state.stream.take();
        state.cursor.take();
        if state.index + 1 < state.playlist.len() {
            state.index += 1;
            true
        } else {
            state.status = AudioPlaybackStatus::Idle;
            false
        }
    };
    if has_next {
        if let Err(error) = start_current(&inner) {
            let _ = inner.events.send(AudioPlaybackEvent::Failed {
                id: Some(item_id),
                message: error.to_string(),
            });
            stop_inner(&inner, true);
        }
    } else {
        let _ = inner.events.send(AudioPlaybackEvent::Finished(item_id));
        emit_snapshot(&inner);
    }
}

fn stop_inner(inner: &PlayerInner, clear_playlist: bool) {
    let mut state = inner.state.lock().expect("player state poisoned");
    if let Some(stream) = state.stream.take() {
        let _ = stream.pause();
    }
    state.cursor = None;
    state.status = AudioPlaybackStatus::Idle;
    if clear_playlist {
        state.playlist.clear();
        state.index = 0;
    }
    drop(state);
    emit_snapshot(inner);
}

fn snapshot_for(state: &PlayerState) -> AudioPlaybackSnapshot {
    let item = state.playlist.get(state.index).cloned();
    let (position_millis, decoded_duration) = state.cursor.as_ref().map_or((0, 0), |cursor| {
        let buffer = cursor.buffer.lock().expect("player cursor poisoned");
        let channels = usize::from(cursor.audio.channels);
        let rendered_frames = buffer.samples.len() / channels;
        let original_frames = cursor.audio.samples.len() / channels;
        let progress = if rendered_frames == 0 {
            0.0
        } else {
            buffer.frame / rendered_frames as f64
        };
        let position = (progress * original_frames as f64 * 1_000.0
            / cursor.audio.sample_rate as f64)
            .round() as u64;
        let duration = original_frames as u64 * 1_000 / u64::from(cursor.audio.sample_rate);
        (position, duration)
    });
    AudioPlaybackSnapshot {
        status: state.status,
        duration_millis: item.as_ref().map_or(decoded_duration, |value| {
            value.duration_millis.max(decoded_duration)
        }),
        item,
        position_millis,
        speed: state.speed,
    }
}

fn emit_snapshot(inner: &PlayerInner) {
    let snapshot = snapshot_for(&inner.state.lock().expect("player state poisoned"));
    let _ = inner.events.send(AudioPlaybackEvent::Changed(snapshot));
}

fn stretch_audio(audio: &DecodedAudio, speed: f64) -> MediaResult<Vec<f32>> {
    if (speed - 1.0).abs() < f64::EPSILON {
        return Ok(audio.samples.clone());
    }
    wsola::stretch(
        &audio.samples,
        audio.sample_rate,
        audio.channels,
        speed as f32,
    )
    .map_err(|error| MediaError::TimeStretch(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_stretch_changes_duration_without_shifting_pitch() {
        let sample_rate = 48_000;
        let frequency = 440.0;
        let samples = (0..sample_rate * 2)
            .map(|index| {
                (index as f32 * frequency * std::f32::consts::TAU / sample_rate as f32).sin()
            })
            .collect::<Vec<_>>();
        let audio = DecodedAudio {
            samples,
            sample_rate,
            channels: 1,
        };

        let stretched = stretch_audio(&audio, 2.0).expect("time stretch should succeed");

        let expected_length = audio.samples.len() / 2;
        assert!(
            stretched.len().abs_diff(expected_length) < sample_rate as usize / 10,
            "2x output length {} should be close to {expected_length}",
            stretched.len()
        );
        let frequency = dominant_frequency(&stretched, sample_rate);
        assert!(
            (frequency - 440.0).abs() < 22.0,
            "2x playback shifted pitch to {frequency} Hz"
        );
    }

    fn dominant_frequency(samples: &[f32], sample_rate: u32) -> f32 {
        let start = samples.len() / 8;
        let end = samples.len() - start;
        let samples = &samples[start..end];
        let crossings = samples
            .windows(2)
            .filter(|pair| (pair[0] <= 0.0 && pair[1] > 0.0) || (pair[0] >= 0.0 && pair[1] < 0.0))
            .count();
        crossings as f32 * sample_rate as f32 / (2.0 * samples.len() as f32)
    }
}
