//! Bounded inline image and BlurHash placeholders, shared by all presentation clients.
use std::{collections::VecDeque, io::Cursor, sync::Mutex};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{codecs::png::PngEncoder, ImageEncoder as _};

use crate::{MediaError, MediaResult};

const MAX_IMAGE_BYTES: usize = 512 * 1024;
const MAX_INPUT_BYTES: usize = MAX_IMAGE_BYTES * 4 / 3 + 128;
const MAX_CACHE_BYTES: usize = 4 * 1024 * 1024;
const MAX_PIXELS: u64 = 1024 * 1024;
type Entries = VecDeque<(String, Option<String>)>;
static CACHE: Mutex<Entries> = Mutex::new(VecDeque::new());
static WORKERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

/// Malformed and unsupported placeholders are absent, never a substitute source URL.
pub async fn image(data: String) -> MediaResult<Option<String>> {
    if data.is_empty() || data.len() > MAX_INPUT_BYTES {
        return Ok(None);
    }
    {
        let mut cache = CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(at) = cache.iter().position(|(source, _)| source == &data) {
            let entry = cache.remove(at).expect("located thumbnail");
            let result = entry.1.clone();
            cache.push_back(entry);
            return Ok(result);
        }
    }
    let permit = WORKERS.acquire().await.map_err(invalid)?;
    crate::blocking(move || {
        let _permit = permit;
        let result = decode(&data);
        let mut cache = CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.retain(|(source, _)| source != &data);
        let mut bytes: usize = cache.iter().map(entry_bytes).sum();
        let entry = (data, result.clone());
        let added = entry_bytes(&entry);
        while bytes + added > MAX_CACHE_BYTES || cache.len() >= 64 {
            let Some(old) = cache.pop_front() else { break };
            bytes -= entry_bytes(&old);
        }
        cache.push_back(entry);
        result
    })
    .await
    .map_err(invalid)
}

fn entry_bytes(entry: &(String, Option<String>)) -> usize {
    entry.0.len() + entry.1.as_ref().map_or(0, String::len)
}

fn invalid(error: impl std::fmt::Display) -> MediaError {
    MediaError::InvalidAttachment(format!("thumbnail processing failed: {error}"))
}

fn decode(data: &str) -> Option<String> {
    let data = data.trim();
    if data.is_ascii() && (6..=166).contains(&data.len()) {
        if let Ok(pixels) = blurhash::decode(data, 32, 32, 1.0) {
            return png(&pixels, 32, 32);
        }
    }
    let encoded = if data.starts_with("data:") {
        let (kind, encoded) = data.split_once(',')?;
        if !matches!(
            kind,
            "data:image/png;base64"
                | "data:image/jpeg;base64"
                | "data:image/gif;base64"
                | "data:image/webp;base64"
                | "data:image/bmp;base64"
        ) {
            return None;
        }
        encoded
    } else {
        data
    };
    let bytes = STANDARD.decode(encoded).ok()?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let mut reader = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?;
    let dimensions = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    if dimensions.0 == 0
        || dimensions.1 == 0
        || dimensions.0 > 2048
        || dimensions.1 > 2048
        || u64::from(dimensions.0) * u64::from(dimensions.1) > MAX_PIXELS
    {
        return None;
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(MAX_PIXELS * 8);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let image = if image.width() > 320 || image.height() > 320 {
        image.thumbnail(320, 320)
    } else {
        image
    }
    .to_rgba8();
    png(image.as_raw(), image.width(), image.height())
}

fn png(pixels: &[u8], width: u32, height: u32) -> Option<String> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(pixels, width, height, image::ExtendedColorType::Rgba8)
        .ok()?;
    Some(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(uri: &str) -> image::RgbaImage {
        image::load_from_memory(
            &STANDARD
                .decode(uri.strip_prefix("data:image/png;base64,").unwrap())
                .unwrap(),
        )
        .unwrap()
        .to_rgba8()
    }

    fn run(future: impl std::future::Future<Output = ()>) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future);
    }

    #[test]
    fn inline_rgba_and_blurhash_preserve_actual_color_and_cached_results() {
        run(async {
            let source = png(&[255, 0, 0, 128, 0, 255, 0, 255], 2, 1).unwrap();
            let actual = image(source.clone()).await.unwrap().unwrap();
            assert_eq!(pixels(&actual).dimensions(), (2, 1));
            assert_eq!(pixels(&actual).into_raw(), [255, 0, 0, 128, 0, 255, 0, 255]);
            assert_eq!(
                image(source).await.unwrap().as_deref(),
                Some(actual.as_str())
            );
            let red = image("00TI:j".into()).await.unwrap().unwrap();
            let red = pixels(&red);
            assert_eq!(red.dimensions(), (32, 32));
            let color = red.get_pixel(0, 0).0;
            assert!(
                color[0] >= 254 && color[1] <= 1 && color[2] <= 1 && color[3] == 255,
                "red BlurHash pixel: {color:?}"
            );
            assert!(red.pixels().all(|pixel| pixel.0 == red.get_pixel(0, 0).0));
        });
    }

    #[test]
    fn malformed_external_oversize_and_pixel_bomb_placeholders_are_absent() {
        run(async {
            for value in [
                "http://example.com/image.png".into(),
                "file:///private/image.png".into(),
                "🥷🥷🥷".into(),
                "invalid".into(),
                "A".repeat(MAX_INPUT_BYTES + 1),
            ] {
                assert_eq!(image(value).await.unwrap(), None);
            }
            let bomb = image::RgbaImage::new(2049, 1);
            let bomb = png(bomb.as_raw(), 2049, 1).unwrap();
            assert_eq!(image(bomb).await.unwrap(), None);
            let too_many_pixels = image::RgbaImage::new(1025, 1025);
            assert_eq!(
                image(png(too_many_pixels.as_raw(), 1025, 1025).unwrap())
                    .await
                    .unwrap(),
                None
            );
        });
    }

    #[test]
    fn valid_large_thumbnail_is_resized_with_its_aspect_ratio() {
        run(async {
            let source = image::RgbaImage::from_pixel(640, 320, image::Rgba([10, 20, 30, 255]));
            let result = image(png(source.as_raw(), 640, 320).unwrap())
                .await
                .unwrap()
                .unwrap();
            let result = pixels(&result);
            assert_eq!(result.dimensions(), (320, 160));
            assert_eq!(result.get_pixel(160, 80).0, [10, 20, 30, 255]);
        });
    }
}
