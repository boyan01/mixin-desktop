use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

const BARS: usize = 32;

/// Bounded capture telemetry. Reading it never locks or copies recorded PCM.
pub(super) struct CaptureProgress {
    sample_rate: u32,
    frames: AtomicU64,
    peaks: [AtomicU8; BARS],
}

/// Owned by the single audio callback, alongside CPAL's mutable callback state.
pub(super) struct ProgressWriter {
    frames: u64,
    peak: u8,
}

impl CaptureProgress {
    pub(super) fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            frames: AtomicU64::new(0),
            peaks: std::array::from_fn(|_| AtomicU8::new(0)),
        }
    }

    pub(super) fn snapshot(&self) -> (u64, Vec<u8>) {
        let frames = self.frames.load(Ordering::Acquire);
        if frames == 0 {
            return (0, vec![0; BARS]);
        }
        let bucket_frames = self.bucket_frames();
        let latest = (frames - 1) / bucket_frames;
        let count = (latest + 1).min(BARS as u64) as usize;
        let mut waveform = vec![0; BARS - count];
        for bucket in latest + 1 - count as u64..=latest {
            waveform.push(self.peaks[bucket as usize % BARS].load(Ordering::Relaxed));
        }
        (
            frames.saturating_mul(1000) / u64::from(self.sample_rate.max(1)),
            waveform,
        )
    }

    fn bucket_frames(&self) -> u64 {
        u64::from((self.sample_rate / 20).max(1))
    }
}

impl ProgressWriter {
    pub(super) fn new() -> Self {
        Self { frames: 0, peak: 0 }
    }

    pub(super) fn record(&mut self, sample: f32, progress: &CaptureProgress) {
        let bucket_frames = progress.bucket_frames();
        let bucket = self.frames / bucket_frames;
        if self.frames.is_multiple_of(bucket_frames) {
            if self.frames != 0 {
                progress.peaks[(bucket as usize - 1) % BARS].store(self.peak, Ordering::Relaxed);
            }
            self.peak = 0;
        }
        let amplitude = if sample.is_finite() {
            (sample.abs().min(1.0) * 255.0).round() as u8
        } else {
            0
        };
        self.peak = self.peak.max(amplitude);
        self.frames += 1;
    }

    pub(super) fn publish(&self, progress: &CaptureProgress) {
        if self.frames != 0 {
            // Publish once per callback, plus completed 50 ms buckets.
            let bucket = (self.frames - 1) / progress.bucket_frames();
            progress.peaks[bucket as usize % BARS].store(self.peak, Ordering::Relaxed);
        }
        progress.frames.store(self.frames, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_chunk_boundaries_preserve_duration_and_recent_peaks() {
        let samples: Vec<_> = (0..300)
            .map(|i| if i % 10 == 0 { 0.8 } else { -0.2 })
            .collect();
        let read = |chunk| {
            let progress = CaptureProgress::new(100);
            let mut writer = ProgressWriter::new();
            for values in samples.chunks(chunk) {
                for sample in values {
                    writer.record(*sample, &progress);
                }
                writer.publish(&progress);
            }
            progress.snapshot()
        };
        let (duration, waveform) = read(1);
        assert_eq!(duration, 3000);
        assert_eq!(waveform.len(), BARS);
        assert_eq!(waveform, [204, 51].repeat(16));
        assert_eq!(read(37), (duration, waveform));
    }

    #[test]
    fn publishing_capture_preserves_silence_padding_and_clamps_peak_amplitude() {
        let progress = CaptureProgress::new(1000);
        let mut writer = ProgressWriter::new();
        for _ in 0..200 {
            writer.record(2.0, &progress);
        }
        assert_eq!(progress.snapshot().0, 0);
        writer.publish(&progress);
        let (duration, waveform) = progress.snapshot();
        assert_eq!(duration, 200);
        assert_eq!(&waveform[..28], &[0; 28]);
        assert_eq!(&waveform[28..], &[255; 4]);
    }
}
