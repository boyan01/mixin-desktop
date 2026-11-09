//! Bounded, software-rendered Lottie stickers. No UI or platform types live here.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::Cursor,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    ImageEncoder as _,
};
use thorvg::{Animation, ColorSpace, MimeType, Paint as _, SwCanvas, Thorvg};
use tokio::sync::{mpsc, oneshot};

use crate::{MediaError, MediaResult};

pub const MAX_STICKER_BYTES: usize = 5 * 1024 * 1024;
const FRAME_CACHE_BYTES: usize = 16 * 1024 * 1024;
const MAX_RENDERERS: usize = 4;
const MAX_NODES: usize = 100_000;
const MAX_DEPTH: usize = 64;
const MAX_DIMENSION: u32 = 512;

type Sources = VecDeque<(String, Arc<[u8]>)>;
static SOURCES: Mutex<Sources> = Mutex::new(VecDeque::new());

pub fn cached_source(url: &str) -> Option<Arc<[u8]>> {
    let mut sources = SOURCES.lock().ok()?;
    let at = sources.iter().position(|(key, _)| key == url)?;
    let entry = sources.remove(at)?;
    let source = entry.1.clone();
    sources.push_back(entry);
    Some(source)
}

pub fn cache_source(url: String, source: Arc<[u8]>) -> MediaResult<()> {
    if source.len() > MAX_STICKER_BYTES {
        return Err(invalid("source is too large"));
    }
    if let Ok(mut sources) = SOURCES.lock() {
        sources.retain(|(key, _)| key != &url);
        let mut bytes: usize = sources.iter().map(|(_, bytes)| bytes.len()).sum();
        while bytes + source.len() > FRAME_CACHE_BYTES || sources.len() >= 32 {
            let Some((_, old)) = sources.pop_front() else {
                break;
            };
            bytes -= old.len();
        }
        sources.push_back((url, source.clone()));
    }
    Ok(())
}

/// A shared source and render key. Dropping it leaves only bounded worker caches.
#[derive(Clone)]
pub struct StickerAnimation {
    key: Arc<str>,
    source: Arc<[u8]>,
    size: u32,
    pub duration: Duration,
    frames: f32,
    pub first_frame: Arc<str>,
}

struct Request {
    key: Arc<str>,
    source: Arc<[u8]>,
    size: u32,
    millis: u64,
    timing: Option<(Duration, f32)>,
    result: oneshot::Sender<MediaResult<(Duration, f32, Arc<str>)>>,
}

fn invalid(reason: impl std::fmt::Display) -> MediaError {
    MediaError::InvalidAttachment(format!("invalid animated sticker: {reason}"))
}

impl StickerAnimation {
    /// Decodes a bounded, self-contained inline composition without a network or account.
    pub async fn from_data_uri(uri: String, size: u32) -> MediaResult<Self> {
        use std::hash::{Hash, Hasher};
        if uri.len() > MAX_STICKER_BYTES.div_ceil(3) * 4 + 80 {
            return Err(invalid("source is too large"));
        }
        let encoded = uri
            .strip_prefix("data:application/json;base64,")
            .or_else(|| uri.strip_prefix("data:application/lottie+json;base64,"))
            .ok_or_else(|| invalid("unsupported inline source"))?;
        let bytes = STANDARD.decode(encoded).map_err(invalid)?;
        // Content identity shares renderers across views, not durable business state.
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hash);
        Self::load(
            format!("inline:{}:{:x}", bytes.len(), hash.finish()),
            bytes.into(),
            size,
        )
        .await
    }

    pub async fn load(key: String, source: Arc<[u8]>, size: u32) -> MediaResult<Self> {
        if source.len() > MAX_STICKER_BYTES
            || source.is_empty()
            || !(1..=MAX_DIMENSION).contains(&size)
        {
            return Err(invalid("source or output size exceeds its limit"));
        }
        let key: Arc<str> = format!("{size}:{key}").into();
        let (duration, frames, first_frame) =
            request(key.clone(), source.clone(), size, 0, None).await?;
        Ok(Self {
            key,
            source,
            size,
            duration,
            frames,
            first_frame,
        })
    }

    /// Requests only the current frame. Callers await it before requesting another.
    pub async fn frame(&self, millis: u64) -> MediaResult<Arc<str>> {
        Ok(request(
            self.key.clone(),
            self.source.clone(),
            self.size,
            millis,
            Some((self.duration, self.frames)),
        )
        .await?
        .2)
    }
}

async fn request(
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

fn validate(bytes: &[u8]) -> MediaResult<()> {
    if bytes.len() > MAX_STICKER_BYTES {
        return Err(invalid("source is too large"));
    }
    let json: serde_json::Value = serde_json::from_slice(bytes).map_err(invalid)?;
    let mut pending = vec![(&json, 0_usize)];
    let mut nodes = 0;
    let mut embedded_pixels = 0_u64;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if nodes > MAX_NODES || depth > MAX_DEPTH {
            return Err(invalid("composition is too complex"));
        }
        match value {
            serde_json::Value::Array(values) => {
                pending.extend(values.iter().map(|v| (v, depth + 1)))
            }
            serde_json::Value::Object(values) => {
                // Self-contained vector stickers and embedded data images only.
                // File and URL assets must never reach ThorVG's asset loader.
                if values
                    .get("p")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|p| !p.starts_with("data:image/png;base64,"))
                    || values
                        .get("u")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|u| !u.is_empty())
                    || values.contains_key("fPath")
                {
                    return Err(invalid("external assets are not supported"));
                }
                if let Some(png) = values
                    .get("p")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|p| p.strip_prefix("data:image/png;base64,"))
                {
                    let bytes = STANDARD.decode(png).map_err(invalid)?;
                    let (width, height) = image::ImageReader::new(Cursor::new(bytes))
                        .with_guessed_format()
                        .map_err(invalid)?
                        .into_dimensions()
                        .map_err(invalid)?;
                    embedded_pixels =
                        embedded_pixels.saturating_add(u64::from(width) * u64::from(height));
                    if width > 4096 || height > 4096 || embedded_pixels > 4 * 1024 * 1024 {
                        return Err(invalid("embedded image budget exceeded"));
                    }
                }
                pending.extend(values.values().map(|v| (v, depth + 1)));
            }
            _ => {}
        }
    }
    for key in ["w", "h"] {
        if !json
            .get(key)
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|n| n.is_finite() && n > 0.0 && n <= 4096.0)
        {
            return Err(invalid("invalid composition dimensions"));
        }
    }
    let number = |key: &str| {
        json.get(key)
            .and_then(serde_json::Value::as_f64)
            .filter(|value| value.is_finite())
    };
    let valid_timing = match (number("fr"), number("ip"), number("op")) {
        (Some(rate), Some(first), Some(last)) => {
            rate > 0.0 && rate <= 120.0 && last > first && last - first <= 432_000.0
        }
        _ => false,
    };
    if !valid_timing {
        return Err(invalid("invalid composition timing"));
    }
    validate_compositions(&json)?;
    Ok(())
}

fn validate_compositions(json: &serde_json::Value) -> MediaResult<()> {
    use serde_json::Value;
    let assets: HashMap<_, _> = json
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|asset| {
            asset
                .get("id")
                .and_then(Value::as_str)
                .map(|id| (id, asset))
        })
        .collect();
    fn layers<'a>(
        value: &'a Value,
        assets: &HashMap<&'a str, &'a Value>,
        active: &mut HashSet<&'a str>,
        memo: &mut HashMap<&'a str, (usize, usize)>,
    ) -> MediaResult<(usize, usize)> {
        let mut count = 0_usize;
        let mut depth = 0_usize;
        for layer in value
            .get("layers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            count = count.saturating_add(1);
            if let Some(id) = layer.get("refId").and_then(Value::as_str) {
                if let Some(asset) = assets.get(id) {
                    let (children, nested) = match memo.get(id) {
                        Some(cached) => *cached,
                        None => {
                            if active.len() >= MAX_DEPTH || !active.insert(id) {
                                return Err(invalid("recursive composition"));
                            }
                            let result = layers(asset, assets, active, memo)?;
                            active.remove(id);
                            memo.insert(id, result);
                            result
                        }
                    };
                    count = count.saturating_add(children);
                    depth = depth.max(nested + 1);
                }
            }
            if count > MAX_NODES || depth > MAX_DEPTH {
                return Err(invalid("expanded composition is too complex"));
            }
        }
        Ok((count, depth))
    }
    layers(json, &assets, &mut HashSet::new(), &mut HashMap::new())?;
    Ok(())
}

#[cfg(test)]
mod tests;
