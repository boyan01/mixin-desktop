//! Shared Lottie rendering worker over ThorVG.
use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    ImageEncoder as _,
};
use thorvg::{Animation, ColorSpace, MimeType, Paint as _, SwCanvas, Thorvg};
use tokio::sync::{mpsc, oneshot};

use super::{invalid, validate, FRAME_CACHE_BYTES};
use crate::MediaResult;

const MAX_RENDERERS: usize = 4;

struct Request {
    key: Arc<str>,
    source: Arc<[u8]>,
    size: u32,
    millis: u64,
    timing: Option<(Duration, f32)>,
    result: oneshot::Sender<MediaResult<(Duration, f32, Arc<str>)>>,
}

pub(super) async fn request(
    key: Arc<str>,
    source: Arc<[u8]>,
    size: u32,
    millis: u64,
    timing: Option<(Duration, f32)>,
) -> MediaResult<(Duration, f32, Arc<str>)> {
    static WORKER: OnceLock<mpsc::Sender<Request>> = OnceLock::new();
    let sender = WORKER.get_or_init(|| {
        let (sender, mut receiver) = mpsc::channel::<Request>(4);
        std::thread::Builder::new()
            .name("sticker-render".into())
            .spawn(move || {
                let Ok(engine) = Thorvg::init() else { return };
                let mut renderers = VecDeque::<(Arc<str>, Raster<'_>)>::new();
                let mut frames = VecDeque::<(Arc<str>, u32, Arc<str>)>::new();
                let mut cache_bytes = 0_usize;
                while let Some(request) = receiver.blocking_recv() {
                    // A disposed owner does not trigger queued work.
                    if request.result.is_closed() {
                        continue;
                    }
                    // Cached frames need no parsed renderer. A grid with more
                    // visible stickers than the decoder LRU must stay cheap.
                    if let Some((duration, count)) = request.timing {
                        let index = frame_at(request.millis, duration, count);
                        if let Some(pixels) = cached_frame(&mut frames, &request.key, index) {
                            let _ = request.result.send(Ok((duration, count, pixels)));
                            continue;
                        }
                    }
                    let result = (|| {
                        let at = renderers.iter().position(|(key, _)| key == &request.key);
                        let (key, mut raster) = match at {
                            Some(at) => renderers.remove(at).expect("located renderer"),
                            None => (
                                request.key.clone(),
                                Raster::new(&engine, &request.source, request.size)?,
                            ),
                        };
                        let duration = raster.duration;
                        let count = raster.frames;
                        let frame = frame_at(request.millis, duration, count);
                        let pixels = match cached_frame(&mut frames, &key, frame) {
                            Some(pixels) => Ok(pixels),
                            None => raster.render(frame).inspect(|pixels| {
                                while cache_bytes + pixels.len() > FRAME_CACHE_BYTES {
                                    let Some((_, _, old)) = frames.pop_front() else {
                                        break;
                                    };
                                    cache_bytes -= old.len();
                                }
                                cache_bytes += pixels.len();
                                frames.push_back((key.clone(), frame, pixels.clone()));
                            }),
                        };
                        renderers.push_back((key, raster));
                        while renderers.len() > MAX_RENDERERS {
                            renderers.pop_front();
                        }
                        pixels.map(|pixels| (duration, count, pixels))
                    })();
                    let _ = request.result.send(result);
                }
            })
            .expect("start sticker renderer");
        sender
    });
    let (result, receiver) = oneshot::channel();
    sender
        .send(Request {
            key,
            source,
            size,
            millis,
            timing,
            result,
        })
        .await
        .map_err(|_| invalid("renderer stopped"))?;
    receiver.await.map_err(|_| invalid("renderer stopped"))?
}

fn frame_at(millis: u64, duration: Duration, count: f32) -> u32 {
    ((millis as f64 / duration.as_secs_f64() / 1000.0).fract() * f64::from(count)).floor() as u32
}

type Frames = VecDeque<(Arc<str>, u32, Arc<str>)>;

fn cached_frame(frames: &mut Frames, key: &str, index: u32) -> Option<Arc<str>> {
    let at = frames
        .iter()
        .position(|(entry, frame, _)| entry.as_ref() == key && *frame == index)?;
    let entry = frames.remove(at)?;
    let pixels = entry.2.clone();
    frames.push_back(entry);
    Some(pixels)
}

// Canvas is dropped before its fixed pixel allocation and the borrowed engine.
struct Raster<'a> {
    canvas: SwCanvas<'a>,
    animation: Animation<'a>,
    pixels: Vec<u32>,
    size: u32,
    frames: f32,
    duration: Duration,
}

impl<'a> Raster<'a> {
    fn new(engine: &'a Thorvg, bytes: &[u8], size: u32) -> MediaResult<Self> {
        validate(bytes)?;
        let mut animation = engine.animation().map_err(invalid)?;
        animation
            .picture_mut()
            .load_data(bytes, MimeType::Lottie, None)
            .map_err(invalid)?;
        let (width, height) = animation.picture().size().map_err(invalid)?;
        let scale = (size as f32 / width).min(size as f32 / height);
        animation
            .picture_mut()
            .set_size(size as f32, size as f32)
            .map_err(invalid)?;
        animation
            .picture_mut()
            .translate(
                (size as f32 - width * scale) / 2.0,
                (size as f32 - height * scale) / 2.0,
            )
            .map_err(invalid)?;
        let frames = animation.total_frame().map_err(invalid)?;
        let duration = animation.duration().map_err(invalid)?;
        if !frames.is_finite()
            || frames < 1.0
            || !duration.is_finite()
            || !(0.001..=3600.0).contains(&duration)
        {
            return Err(invalid("invalid animation timing"));
        }
        let mut pixels = vec![0_u32; (size * size) as usize];
        let mut canvas = engine.sw_canvas(Default::default()).map_err(invalid)?;
        // SAFETY: this allocation is never resized or moved out. `canvas` drops
        // before `pixels`; all accesses are serialized on this worker thread.
        unsafe { canvas.set_target(&mut pixels, size, size, size, ColorSpace::ABGR8888) }
            .map_err(invalid)?;
        Ok(Self {
            canvas,
            animation,
            pixels,
            size,
            frames,
            duration: Duration::from_secs_f32(duration),
        })
    }

    fn render(&mut self, frame: u32) -> MediaResult<Arc<str>> {
        if (self.animation.frame().map_err(invalid)? - frame as f32).abs() > 0.001 {
            self.animation.set_frame(frame as f32).map_err(invalid)?;
        }
        self.canvas.clear().map_err(invalid)?;
        self.pixels.fill(0);
        self.canvas
            .add(
                self.animation
                    .picture()
                    .duplicate()
                    .ok_or_else(|| invalid("cannot copy frame"))?,
            )
            .map_err(invalid)?;
        self.canvas.draw(true).map_err(invalid)?;
        self.canvas.sync().map_err(invalid)?;
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            let [r, g, b, a] = pixel.to_le_bytes();
            // ThorVG's target is premultiplied; PNG stores straight alpha.
            let straight = |c: u8| {
                if a == 0 {
                    0
                } else {
                    (u32::from(c) * 255 / u32::from(a)).min(255) as u8
                }
            };
            rgba.extend_from_slice(&[straight(r), straight(g), straight(b), a]);
        }
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Up)
            .write_image(&rgba, self.size, self.size, image::ExtendedColorType::Rgba8)
            .map_err(invalid)?;
        Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)).into())
    }
}
