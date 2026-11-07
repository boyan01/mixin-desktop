use std::fs;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    // The low bit marks a pending preparation. Invalidating never waits for CPAL.
    revision: AtomicU64,
    closed: AtomicBool,
    events: broadcast::Sender<AudioPlaybackEvent>,
}

struct PlayerState {
    status: AudioPlaybackStatus,
    playlist: Vec<AudioPlaybackItem>,
    index: usize,
    stream: Option<Stream>,
    cursor: Option<Arc<PlaybackCursor>>,
    speed: f64,
    revision: u64,
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
                    revision: 0,
                }),
                revision: AtomicU64::new(0),
                closed: AtomicBool::new(false),
                events,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AudioPlaybackEvent> {
        self.inner.events.subscribe()
    }

    pub fn play(&self, playlist: Vec<AudioPlaybackItem>, start_index: u64) -> MediaResult<()> {
        self.play_with_read(playlist, start_index, |path| fs::read(path))
    }

    /// Reserves before entering the blocking pool. Dropping the returned future
    /// invalidates preparation even when its blocking worker has already started.
    pub fn play_async(
        &self,
        playlist: Vec<AudioPlaybackItem>,
        start_index: u64,
    ) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        let request = validate_playlist(&playlist, start_index).and_then(|_| self.reserve());
        async move {
            let mut request = request?;
            let inner = request.inner.clone();
            let index = start_index as usize;
            let revision = request.revision;
            let result = tokio::task::spawn_blocking(move || {
                play_reserved(&inner, playlist, index, revision, |path| fs::read(path))
            })
            .await
            .map_err(|error| MediaError::Stream(format!("playback worker failed: {error}")))?;
            request.completed = result.is_ok();
            drop(request);
            result
        }
    }

    fn reserve(&self) -> MediaResult<PlaybackRequest> {
        if self.inner.closed.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        Ok(PlaybackRequest::new(self.inner.clone()))
    }

    fn play_with_read(
        &self,
        playlist: Vec<AudioPlaybackItem>,
        start_index: u64,
        read: impl FnOnce(&str) -> std::io::Result<Vec<u8>>,
    ) -> MediaResult<()> {
        validate_playlist(&playlist, start_index)?;
        let mut request = self.reserve()?;
        let result = play_reserved(
            &request.inner,
            playlist,
            start_index as usize,
            request.revision,
            read,
        );
        request.completed = result.is_ok();
        result
    }

    /// Immediately rejects new work; device disposal runs only when awaited.
    pub fn close(&self) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        self.inner.closed.store(true, Ordering::Release);
        self.stop_async()
    }

    /// Invalidates immediately and releases the captured playback on a worker.
    /// A later successful play is never stopped by delayed cleanup.
    pub fn stop_async(&self) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        let mut request = PlaybackRequest::new(self.inner.clone());
        async move {
            let inner = request.inner.clone();
            let revision = request.revision;
            tokio::task::spawn_blocking(move || {
                let mut state = inner.state.lock().expect("player state poisoned");
                if state.revision > revision {
                    return;
                }
                let stream = clear_playback(&mut state, true);
                let _ = inner
                    .events
                    .send(AudioPlaybackEvent::Changed(snapshot_for(&state)));
                drop(state);
                drop(stream);
            })
            .await
            .map_err(|error| MediaError::Stream(format!("playback cleanup failed: {error}")))?;
            let _ = request.inner.revision.compare_exchange(
                request.revision,
                request.revision & !1,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            request.completed = true;
            drop(request);
            Ok(())
        }
    }

    pub fn pause(&self) -> MediaResult<()> {
        ensure_open(&self.inner)?;
        invalidate(&self.inner);
        let mut state = self.inner.state.lock().expect("player state poisoned");
        ensure_open(&self.inner)?;
        invalidate(&self.inner);
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
        ensure_open(&self.inner)?;
        invalidate(&self.inner);
        let mut state = self.inner.state.lock().expect("player state poisoned");
        ensure_open(&self.inner)?;
        invalidate(&self.inner);
        if let Some(cursor) = &state.cursor {
            cursor.advance_scheduled.store(false, Ordering::Release);
        }
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
        invalidate(&self.inner);
        let state = self.inner.state.lock().expect("player state poisoned");
        invalidate(&self.inner);
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
        ensure_open(&self.inner)?;
        if !(0.5..=2.0).contains(&speed) {
            return Err(MediaError::DeviceConfiguration(format!(
                "playback speed {speed} is outside 0.5...2.0"
            )));
        }
        invalidate(&self.inner);
        let mut state = self.inner.state.lock().expect("player state poisoned");
        ensure_open(&self.inner)?;
        if (state.speed - speed).abs() < f64::EPSILON {
            return Ok(());
        }
        invalidate(&self.inner);
        if let Some(cursor) = &state.cursor {
            cursor.advance_scheduled.store(false, Ordering::Release);
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
        invalidate(&self.inner);
        stop_inner(&self.inner, true);
    }

    pub fn snapshot(&self) -> AudioPlaybackSnapshot {
        snapshot_for(&self.inner.state.lock().expect("player state poisoned"))
    }
}

struct PlaybackRequest {
    inner: Arc<PlayerInner>,
    revision: u64,
    completed: bool,
}

impl PlaybackRequest {
    fn new(inner: Arc<PlayerInner>) -> Self {
        let previous = inner
            .revision
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |revision| {
                Some(revision.wrapping_add(2) | 1)
            })
            .expect("revision update always succeeds");
        Self {
            inner,
            revision: previous.wrapping_add(2) | 1,
            completed: false,
        }
    }
}

fn validate_playlist(playlist: &[AudioPlaybackItem], start_index: u64) -> MediaResult<()> {
    if playlist.is_empty() || start_index as usize >= playlist.len() {
        Err(MediaError::MalformedAudio(
            "playback list or start index is invalid".into(),
        ))
    } else {
        Ok(())
    }
}

impl Drop for PlaybackRequest {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        if self
            .inner
            .revision
            .compare_exchange(
                self.revision,
                self.revision.wrapping_add(2) & !1,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            return;
        }
        // A finished or superseded request usually owns no published resource.
        // Never wait here if a worker is still committing under the state lock.
        if self
            .inner
            .state
            .try_lock()
            .is_ok_and(|state| state.revision != self.revision)
        {
            return;
        }
        // The worker may have published before its awaiter received the result.
        // Dispose only this request's stream, preserving every later request.
        let inner = self.inner.clone();
        let revision = self.revision;
        std::thread::spawn(move || {
            let mut state = inner.state.lock().expect("player state poisoned");
            if state.revision != revision {
                return;
            }
            let _ = inner.revision.compare_exchange(
                revision & !1,
                revision.wrapping_add(2) & !1,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            let stream = clear_playback(&mut state, true);
            let _ = inner
                .events
                .send(AudioPlaybackEvent::Changed(snapshot_for(&state)));
            drop(state);
            drop(stream);
        });
    }
}

fn cancelled() -> MediaError {
    MediaError::Stream("playback request was cancelled or superseded".into())
}

fn ensure_open(inner: &PlayerInner) -> MediaResult<()> {
    if inner.closed.load(Ordering::Acquire) {
        Err(cancelled())
    } else {
        Ok(())
    }
}

fn invalidate(inner: &PlayerInner) {
    inner
        .revision
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |revision| {
            Some(revision.wrapping_add(2) & !1)
        })
        .expect("revision update always succeeds");
}

fn is_pending(inner: &PlayerInner, revision: u64) -> bool {
    !inner.closed.load(Ordering::Acquire) && inner.revision.load(Ordering::Acquire) == revision
}

fn play_reserved(
    inner: &Arc<PlayerInner>,
    playlist: Vec<AudioPlaybackItem>,
    index: usize,
    revision: u64,
    read: impl FnOnce(&str) -> std::io::Result<Vec<u8>>,
) -> MediaResult<()> {
    if !is_pending(inner, revision) {
        return Err(cancelled());
    }
    let speed = inner.state.lock().expect("player state poisoned").speed;
    let result = start_playlist(inner, playlist, index, revision, speed, read);
    if result.is_err() && is_pending(inner, revision) {
        let state = inner.state.lock().expect("player state poisoned");
        if let Some(cursor) = &state.cursor {
            cursor.advance_scheduled.store(false, Ordering::Release);
        }
    }
    result
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Preparation does not publish the requested item. Only a still-current
/// request can atomically replace its queue and the resources playing it.
fn start_playlist(
    inner: &Arc<PlayerInner>,
    playlist: Vec<AudioPlaybackItem>,
    index: usize,
    revision: u64,
    speed: f64,
    read: impl FnOnce(&str) -> std::io::Result<Vec<u8>>,
) -> MediaResult<()> {
    if !is_pending(inner, revision) {
        return Err(cancelled());
    }
    let item = &playlist[index];
    let bytes = read(&item.path)?;
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
    // Avoid opening a device for work that was cancelled while decoding.
    if !is_pending(inner, revision) {
        return Err(MediaError::Stream("playback request was superseded".into()));
    }
    let stream = build_output_stream(cursor.clone(), Arc::downgrade(inner), item.id.clone())?;
    let mut state = inner.state.lock().expect("player state poisoned");
    if !is_pending(inner, revision) {
        drop(state);
        drop(stream);
        return Err(MediaError::Stream("playback request was superseded".into()));
    }
    stream
        .play()
        .map_err(|error| MediaError::Stream(error.to_string()))?;
    if inner
        .revision
        .compare_exchange(revision, revision & !1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
        || inner.closed.load(Ordering::Acquire)
    {
        drop(state);
        drop(stream);
        return Err(cancelled());
    }
    let old_stream = state.stream.replace(stream);
    state.cursor = Some(cursor);
    state.playlist = playlist;
    state.index = index;
    state.status = AudioPlaybackStatus::Playing;
    state.revision = revision;
    let _ = inner
        .events
        .send(AudioPlaybackEvent::Changed(snapshot_for(&state)));
    drop(state);
    // CPAL may wait for callbacks when dropping a stream. Callbacks can inspect
    // the player, so release the state lock before disposing the previous one.
    drop(old_stream);
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
    let error_cursor = Arc::downgrade(&cursor);
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                fill_output(output, output_channels, output_rate, &cursor);
                if cursor.ended.load(Ordering::Acquire)
                    && inner.upgrade().is_some_and(|inner| {
                        !inner.closed.load(Ordering::Acquire)
                            && inner.revision.load(Ordering::Acquire) & 1 == 0
                    })
                    && !cursor.advance_scheduled.swap(true, Ordering::AcqRel)
                {
                    let inner = inner.clone();
                    let cursor = Arc::downgrade(&cursor);
                    std::thread::spawn(move || advance(inner, cursor));
                }
            },
            move |error| {
                let inner = error_inner.clone();
                let cursor = error_cursor.clone();
                let id = error_item_id.clone();
                let message = error.to_string();
                // A backend can invoke this while a control call is waiting
                // for its callback. Never acquire the player lock on that thread.
                std::thread::spawn(move || {
                    if let Some(inner) = inner.upgrade() {
                        let state = inner.state.lock().expect("player state poisoned");
                        if is_current_cursor(&state, &cursor) {
                            let _ = inner.events.send(AudioPlaybackEvent::Failed {
                                id: Some(id),
                                message,
                            });
                        }
                    }
                });
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

fn is_current_cursor(state: &PlayerState, cursor: &Weak<PlaybackCursor>) -> bool {
    state
        .cursor
        .as_ref()
        .is_some_and(|current| Weak::ptr_eq(&Arc::downgrade(current), cursor))
}

fn advance(inner: Weak<PlayerInner>, cursor: Weak<PlaybackCursor>) {
    let Some(inner) = inner.upgrade() else {
        return;
    };
    let (playlist, index, revision, speed) = {
        let mut state = inner.state.lock().expect("player state poisoned");
        let previous = inner.revision.load(Ordering::Acquire);
        if !is_current_cursor(&state, &cursor)
            || inner.closed.load(Ordering::Acquire)
            || previous & 1 != 0
            || state.status != AudioPlaybackStatus::Playing
            || !state
                .cursor
                .as_ref()
                .is_some_and(|cursor| cursor.ended.load(Ordering::Acquire))
        {
            if let Some(cursor) = cursor.upgrade() {
                cursor.advance_scheduled.store(false, Ordering::Release);
            }
            return;
        }
        let revision = previous.wrapping_add(2) | 1;
        if inner
            .revision
            .compare_exchange(previous, revision, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        if state.index + 1 >= state.playlist.len() {
            inner
                .revision
                .compare_exchange(revision, revision & !1, Ordering::AcqRel, Ordering::Acquire)
                .ok();
            let item_id = state.playlist[state.index].id.clone();
            let stream = state.stream.take();
            state.cursor.take();
            state.status = AudioPlaybackStatus::Idle;
            let _ = inner.events.send(AudioPlaybackEvent::Finished(item_id));
            let _ = inner
                .events
                .send(AudioPlaybackEvent::Changed(snapshot_for(&state)));
            drop(state);
            drop(stream);
            return;
        }
        (
            state.playlist.clone(),
            state.index + 1,
            revision,
            state.speed,
        )
    };
    let next_id = playlist[index].id.clone();
    if let Err(error) = start_playlist(&inner, playlist, index, revision, speed, |path| {
        fs::read(path)
    }) {
        let mut state = inner.state.lock().expect("player state poisoned");
        if !is_pending(&inner, revision) || !is_current_cursor(&state, &cursor) {
            return;
        }
        invalidate(&inner);
        let stream = clear_playback(&mut state, true);
        let _ = inner.events.send(AudioPlaybackEvent::Failed {
            id: Some(next_id),
            message: error.to_string(),
        });
        let _ = inner
            .events
            .send(AudioPlaybackEvent::Changed(snapshot_for(&state)));
        drop(state);
        drop(stream);
    }
}

fn clear_playback(state: &mut PlayerState, clear_playlist: bool) -> Option<Stream> {
    let stream = state.stream.take();
    state.cursor = None;
    state.status = AudioPlaybackStatus::Idle;
    if clear_playlist {
        state.playlist.clear();
        state.index = 0;
    }
    stream
}

fn stop_inner(inner: &PlayerInner, clear_playlist: bool) {
    let mut state = inner.state.lock().expect("player state poisoned");
    invalidate(inner);
    let stream = clear_playback(&mut state, clear_playlist);
    let _ = inner
        .events
        .send(AudioPlaybackEvent::Changed(snapshot_for(&state)));
    drop(state);
    drop(stream);
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
mod tests;
