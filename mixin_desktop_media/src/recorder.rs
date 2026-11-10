use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, Weak};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::codec::{encode_voice, VOICE_SAMPLE_RATE};
use crate::{MediaError, MediaResult};

const MAX_RECORDING_DURATION: Duration = Duration::from_secs(60);
const WAVEFORM_SAMPLES: usize = 100;

mod progress;
use progress::{CaptureProgress, ProgressWriter};

mod state;
use state::RecordedFile;
pub use state::{VoiceRecorderEvent, VoiceRecorderSnapshot, VoiceRecorderStatus, VoiceRecording};

#[derive(Clone)]
pub struct VoiceRecorder {
    inner: Arc<RecorderInner>,
}

struct RecorderInner {
    operation: Mutex<()>,
    closed: AtomicBool,
    state: Mutex<RecorderState>,
    events: broadcast::Sender<VoiceRecorderEvent>,
    next_generation: Mutex<u64>,
}

enum RecorderState {
    Idle,
    Active(ActiveRecording),
    Recorded(VoiceRecording),
}

struct ActiveRecording {
    stream: Stream,
    capture: Arc<CaptureBuffer>,
    path: PathBuf,
    generation: u64,
    cancel_timer: mpsc::Sender<()>,
}

struct CaptureBuffer {
    samples: Mutex<Vec<f32>>,
    sample_rate: u32,
    max_samples: usize,
    progress: CaptureProgress,
}

impl VoiceRecorder {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(32);
        Self {
            inner: Arc::new(RecorderInner {
                operation: Mutex::new(()),
                closed: AtomicBool::new(false),
                state: Mutex::new(RecorderState::Idle),
                events,
                next_generation: Mutex::new(0),
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<VoiceRecorderEvent> {
        self.inner.events.subscribe()
    }

    pub fn snapshot(&self) -> VoiceRecorderSnapshot {
        snapshot_for(&self.inner.state.lock().expect("recorder state poisoned"))
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Acquire)
    }

    pub fn start(&self) -> MediaResult<()> {
        self.start_session().map(|_| ())
    }

    pub fn start_session(&self) -> MediaResult<u64> {
        let _operation = self.inner.operation.lock().expect("recorder operation");
        self.ensure_open()?;
        {
            let state = self.inner.state.lock().expect("recorder state poisoned");
            if matches!(*state, RecorderState::Active(_)) {
                return Err(MediaError::RecorderActive);
            }
        }
        self.cancel_locked()?;

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(MediaError::MissingDevice("input"))?;
        let supported = device
            .default_input_config()
            .map_err(|error| MediaError::DeviceConfiguration(error.to_string()))?;
        let config: StreamConfig = supported.into();
        let channels = usize::from(config.channels);
        let capture = Arc::new(CaptureBuffer {
            samples: Mutex::new(Vec::with_capacity(
                supported.sample_rate() as usize * usize::from(config.channels),
            )),
            sample_rate: supported.sample_rate(),
            max_samples: supported.sample_rate() as usize
                * MAX_RECORDING_DURATION.as_secs() as usize,
            progress: CaptureProgress::new(supported.sample_rate()),
        });
        let stream = build_input_stream(
            &device,
            config,
            supported.sample_format(),
            channels,
            capture.clone(),
            Arc::downgrade(&self.inner),
        )?;
        self.ensure_open()?;
        stream
            .play()
            .map_err(|error| MediaError::Stream(error.to_string()))?;

        let path = recording_path()?;
        let generation = {
            let mut value = self
                .inner
                .next_generation
                .lock()
                .expect("recorder generation poisoned");
            *value += 1;
            *value
        };
        let (cancel_timer, timer) = mpsc::channel();
        {
            let mut state = self.inner.state.lock().expect("recorder state poisoned");
            self.ensure_open()?;
            *state = RecorderState::Active(ActiveRecording {
                stream,
                capture,
                path,
                generation,
                cancel_timer,
            });
        }
        emit_snapshot(&self.inner);

        let inner = Arc::downgrade(&self.inner);
        std::thread::spawn(move || {
            if timer.recv_timeout(MAX_RECORDING_DURATION).is_err() {
                let Some(inner) = inner.upgrade() else {
                    return;
                };
                if let Err(error) = finish_recording(&inner, Some(generation)) {
                    let _ = inner
                        .events
                        .send(VoiceRecorderEvent::Failed(error.to_string()));
                }
            }
        });
        Ok(generation)
    }

    /// Marks the session closed without waiting for an in-flight device operation.
    pub fn close(&self) -> impl Future<Output = MediaResult<()>> + Send + 'static {
        self.inner.closed.store(true, Ordering::Release);
        let recorder = self.clone();
        async move {
            tokio::task::spawn_blocking(move || recorder.cancel())
                .await
                .map_err(|error| MediaError::Stream(format!("recording cleanup failed: {error}")))?
        }
    }

    fn ensure_open(&self) -> MediaResult<()> {
        if self.inner.closed.load(Ordering::Acquire) {
            Err(MediaError::Stream("recording session is closed".into()))
        } else {
            Ok(())
        }
    }

    pub fn stop(&self) -> MediaResult<VoiceRecording> {
        finish_recording(&self.inner, None)
    }

    pub fn stop_session(&self, session_id: u64) -> MediaResult<VoiceRecording> {
        finish_recording(&self.inner, Some(session_id))
    }

    pub fn cancel_session(&self, session_id: u64) -> MediaResult<()> {
        let _operation = self.inner.operation.lock().expect("recorder operation");
        let matches = match &*self.inner.state.lock().expect("recorder state") {
            RecorderState::Active(active) => active.generation == session_id,
            RecorderState::Recorded(recording) => recording.session_id == session_id,
            RecorderState::Idle => false,
        };
        if matches {
            self.cancel_locked()?;
        }
        Ok(())
    }

    pub fn cancel(&self) -> MediaResult<()> {
        let _operation = self.inner.operation.lock().expect("recorder operation");
        self.cancel_locked()
    }

    fn cancel_locked(&self) -> MediaResult<()> {
        let previous = {
            let mut state = self.inner.state.lock().expect("recorder state poisoned");
            std::mem::replace(&mut *state, RecorderState::Idle)
        };
        match previous {
            RecorderState::Active(active) => {
                let _ = active.cancel_timer.send(());
                let _ = active.stream.pause();
                drop(active.stream);
                remove_file_if_present(&active.path)?;
            }
            RecorderState::Recorded(recording) => drop(recording),
            RecorderState::Idle => {}
        }
        emit_snapshot(&self.inner);
        Ok(())
    }
}

impl Default for VoiceRecorder {
    fn default() -> Self {
        Self::new()
    }
}

fn finish_recording(
    inner: &Arc<RecorderInner>,
    required_generation: Option<u64>,
) -> MediaResult<VoiceRecording> {
    let _operation = inner.operation.lock().expect("recorder operation");
    let previous = {
        let mut state = inner.state.lock().expect("recorder state poisoned");
        match &*state {
            RecorderState::Recorded(recording) => {
                if required_generation.is_some_and(|id| id != recording.session_id) {
                    return Err(MediaError::RecorderInactive);
                }
                return Ok(recording.clone());
            }
            RecorderState::Idle => return Err(MediaError::RecorderInactive),
            RecorderState::Active(active)
                if required_generation.is_some_and(|value| value != active.generation) =>
            {
                return Err(MediaError::RecorderInactive);
            }
            RecorderState::Active(_) => {}
        }
        std::mem::replace(&mut *state, RecorderState::Idle)
    };
    let RecorderState::Active(active) = previous else {
        return Err(MediaError::RecorderInactive);
    };
    let _ = active.cancel_timer.send(());
    let _ = active.stream.pause();
    drop(active.stream);

    let native_samples = active
        .capture
        .samples
        .lock()
        .expect("capture samples poisoned")
        .clone();
    let samples = resample_mono(
        &native_samples,
        active.capture.sample_rate,
        VOICE_SAMPLE_RATE,
    );
    if samples.is_empty() {
        remove_file_if_present(&active.path)?;
        return Err(MediaError::EmptyRecording);
    }
    let pcm = samples
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16)
        .collect::<Vec<_>>();
    let encoded = encode_voice(&pcm)?;
    fs::write(&active.path, encoded)?;
    let recording = VoiceRecording {
        session_id: active.generation,
        _file: Some(Arc::new(RecordedFile(active.path.clone()))),
        path: active.path.to_string_lossy().into_owned(),
        duration_millis: pcm.len() as u64 * 1_000 / u64::from(VOICE_SAMPLE_RATE),
        waveform: waveform(&pcm),
    };
    {
        let mut state = inner.state.lock().expect("recorder state poisoned");
        *state = RecorderState::Recorded(recording.clone());
    }
    emit_snapshot(inner);
    Ok(recording)
}

fn build_input_stream(
    device: &cpal::Device,
    config: StreamConfig,
    format: SampleFormat,
    channels: usize,
    capture: Arc<CaptureBuffer>,
    inner: Weak<RecorderInner>,
) -> MediaResult<Stream> {
    macro_rules! build {
        ($sample:ty) => {
            build_typed_input_stream::<$sample>(device, config, channels, capture, inner)
        };
    }
    match format {
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
        _ => Err(MediaError::DeviceConfiguration(format!(
            "unsupported input sample format {format}"
        ))),
    }
}

fn build_typed_input_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    channels: usize,
    capture: Arc<CaptureBuffer>,
    inner: Weak<RecorderInner>,
) -> MediaResult<Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let mut progress = ProgressWriter::new();
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let Ok(mut samples) = capture.samples.try_lock() else {
                    return;
                };
                for frame in data.chunks(channels) {
                    if samples.len() >= capture.max_samples {
                        break;
                    }
                    let sum = frame.iter().copied().map(f32::from_sample).sum::<f32>();
                    let sample = sum / channels as f32;
                    samples.push(sample);
                    progress.record(sample, &capture.progress);
                }
                progress.publish(&capture.progress);
            },
            move |error| {
                if let Some(inner) = inner.upgrade() {
                    let _ = inner
                        .events
                        .send(VoiceRecorderEvent::Failed(error.to_string()));
                }
            },
            None,
        )
        .map_err(|error| MediaError::Stream(error.to_string()))
}

fn resample_mono(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    if input.is_empty() || input_rate == 0 || output_rate == 0 {
        return Vec::new();
    }
    if input_rate == output_rate {
        return input.to_vec();
    }
    let output_len = input.len() as u64 * u64::from(output_rate) / u64::from(input_rate);
    let ratio = input_rate as f64 / output_rate as f64;
    (0..output_len as usize)
        .map(|index| {
            let position = index as f64 * ratio;
            let lower = position.floor() as usize;
            let upper = (lower + 1).min(input.len() - 1);
            let fraction = (position - lower as f64) as f32;
            input[lower] + (input[upper] - input[lower]) * fraction
        })
        .collect()
}

fn waveform(samples: &[i16]) -> Vec<u8> {
    if samples.is_empty() {
        return Vec::new();
    }
    let peaks = (0..WAVEFORM_SAMPLES)
        .map(|index| {
            let lower = index * samples.len() / WAVEFORM_SAMPLES;
            let upper = ((index + 1) * samples.len() / WAVEFORM_SAMPLES)
                .max(lower + 1)
                .min(samples.len());
            samples[lower..upper]
                .iter()
                .map(|sample| sample.unsigned_abs())
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let maximum = peaks.iter().copied().max().unwrap_or(0);
    if maximum == 0 {
        return vec![0; WAVEFORM_SAMPLES];
    }
    peaks
        .into_iter()
        .map(|peak| (u32::from(peak) * 255 / u32::from(maximum)) as u8)
        .collect()
}

fn recording_path() -> MediaResult<PathBuf> {
    let directory = std::env::temp_dir().join("mixin-desktop-voice");
    fs::create_dir_all(&directory)?;
    Ok(directory.join(format!("{}.ogg", Uuid::new_v4())))
}

fn remove_file_if_present(path: &std::path::Path) -> MediaResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn snapshot_for(state: &RecorderState) -> VoiceRecorderSnapshot {
    match state {
        RecorderState::Idle => VoiceRecorderSnapshot {
            session_id: None,
            status: VoiceRecorderStatus::Idle,
            recording: None,
            duration_millis: 0,
            waveform: Vec::new(),
        },
        RecorderState::Active(active) => {
            let (duration_millis, waveform) = active.capture.progress.snapshot();
            VoiceRecorderSnapshot {
                session_id: Some(active.generation),
                status: VoiceRecorderStatus::Recording,
                recording: None,
                duration_millis,
                waveform,
            }
        }
        RecorderState::Recorded(recording) => VoiceRecorderSnapshot {
            session_id: Some(recording.session_id),
            status: VoiceRecorderStatus::Recorded,
            recording: Some(recording.clone()),
            duration_millis: recording.duration_millis,
            waveform: recording.waveform.clone(),
        },
    }
}

fn emit_snapshot(inner: &RecorderInner) {
    let snapshot = snapshot_for(&inner.state.lock().expect("recorder state poisoned"));
    let _ = inner.events.send(VoiceRecorderEvent::Changed(snapshot));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_a_session_rejects_a_queued_start_without_waiting_for_the_operation_lock() {
        let recorder = VoiceRecorder::new();
        let held = recorder.inner.operation.lock().unwrap();
        let queued = recorder.clone();
        let (entered, ready) = mpsc::sync_channel(0);
        let worker = std::thread::spawn(move || {
            entered.send(()).unwrap();
            queued.start_session()
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let closing = recorder.close();
        drop(held);
        assert!(
            matches!(worker.join().unwrap(), Err(MediaError::Stream(message)) if message.contains("closed"))
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(closing).unwrap();
        assert_eq!(recorder.snapshot().status, VoiceRecorderStatus::Idle);
        assert!(recorder.start_session().is_err());
    }

    #[test]
    fn a_detached_send_keeps_its_source_and_cannot_cancel_a_newer_recording() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.ogg");
        let second = directory.path().join("second.ogg");
        fs::write(&first, b"first recording").unwrap();
        fs::write(&second, b"second recording").unwrap();
        let recorder = VoiceRecorder::new();
        let recording = |path: &PathBuf, session_id| VoiceRecording {
            path: path.to_string_lossy().into_owned(),
            session_id,
            duration_millis: 1000,
            waveform: vec![1],
            _file: Some(Arc::new(RecordedFile(path.clone()))),
        };
        *recorder.inner.state.lock().unwrap() = RecorderState::Recorded(recording(&first, 1));
        assert_eq!(recorder.snapshot().duration_millis, 1000);
        assert_eq!(recorder.snapshot().waveform, vec![1]);
        let sending = recorder.snapshot().recording.unwrap();
        recorder.cancel_session(1).unwrap();
        assert_eq!(recorder.snapshot().status, VoiceRecorderStatus::Idle);
        assert_eq!(recorder.snapshot().duration_millis, 0);
        assert!(recorder.snapshot().waveform.is_empty());
        *recorder.inner.state.lock().unwrap() = RecorderState::Recorded(recording(&second, 2));
        recorder.cancel_session(1).unwrap();
        assert_eq!(recorder.snapshot().session_id, Some(2));
        assert!(first.is_file());
        assert!(second.is_file());
        assert!(recorder.stop_session(1).is_err());
        drop(sending);
        assert!(!first.exists());
        assert!(second.exists());
        recorder.cancel_session(2).unwrap();
        assert!(!second.exists());
    }

    #[test]
    fn waveform_always_has_protocol_width_and_normalizes_peak() {
        let samples = (0..1_600)
            .map(|index| if index == 800 { i16::MAX } else { 100 })
            .collect::<Vec<_>>();
        let waveform = waveform(&samples);
        assert_eq!(waveform.len(), 100);
        assert_eq!(waveform.iter().copied().max(), Some(255));
    }

    #[test]
    fn resampling_preserves_duration() {
        let input = vec![0.25; 48_000];
        let output = resample_mono(&input, 48_000, 16_000);
        assert_eq!(output.len(), 16_000);
    }
}
