//! Local attachment preparation shared by desktop presentation clients.

use image::{codecs::jpeg::JpegEncoder, ImageDecoder, ImageFormat, ImageReader};
use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};
#[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
use std::{process::Stdio, time::Duration};
use tempfile::TempPath;

#[cfg(target_os = "macos")]
mod video_macos;

struct VideoInfo {
    width: i32,
    height: i32,
    duration_millis: i64,
    thumbnail: Option<String>,
}

use crate::{MediaError, MediaResult};

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct PreparedAttachment {
    pub path: String,
    pub name: String,
    pub mime_type: String,
    pub kind: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_millis: Option<i64>,
    pub thumbnail: Option<String>,
    pub size: u64,
    // A committed send keeps this handle until the file has been imported.
    temporary: Option<Arc<TempPath>>,
}

impl PreparedAttachment {
    pub(crate) fn retain_temporary(&mut self, path: TempPath) {
        self.temporary = Some(Arc::new(path));
    }
}

pub async fn prepare(path: String, media: bool, compress: bool) -> MediaResult<PreparedAttachment> {
    let mut item = crate::blocking(move || prepare_file(Path::new(&path), media, compress))
        .await
        .map_err(|error| MediaError::InvalidAttachment(error.to_string()))??;
    if media && item.mime_type.starts_with("video/") {
        let info = probe_video(&item.path).await?;
        item.kind = "VIDEO".into();
        item.width = Some(info.width);
        item.height = Some(info.height);
        item.duration_millis = Some(info.duration_millis);
        item.thumbnail = info.thumbnail;
    }
    Ok(item)
}

pub async fn prepare_clipboard_file(
    name: String,
    bytes: Vec<u8>,
    media: bool,
    compress: bool,
) -> MediaResult<PreparedAttachment> {
    const MAX_BYTES: usize = 48 * 1024 * 1024;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(MediaError::InvalidAttachment(
            "The pasted file must be nonempty and no larger than 48 MiB".into(),
        ));
    }
    let source = crate::blocking(move || {
        let name = name.rsplit(['/', '\\']).next().unwrap_or_default();
        let name = if name.is_empty() || matches!(name, "." | "..") {
            "clipboard"
        } else {
            name
        };
        if name.chars().any(char::is_control) || name.len() > 255 {
            return Err(MediaError::InvalidAttachment(
                "The pasted file name is invalid".into(),
            ));
        }
        let extension = Path::new(name)
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| {
                value.len() <= 16 && value.chars().all(|ch| ch.is_ascii_alphanumeric())
            });
        let source = tempfile::Builder::new()
            .prefix("mixin-clipboard-")
            .suffix(
                &extension
                    .map(|value| format!(".{value}"))
                    .unwrap_or_default(),
            )
            .tempfile()?;
        std::fs::write(source.path(), bytes)?;
        Ok::<_, MediaError>((source.into_temp_path(), name.to_owned()))
    })
    .await
    .map_err(|error| MediaError::InvalidAttachment(error.to_string()))??;
    let (source, name) = source;
    let mut item = prepare(source.to_string_lossy().into_owned(), media, compress).await?;
    item.name = if item.temporary.is_some() {
        Path::new(&name)
            .with_extension(Path::new(&item.name).extension().unwrap_or_default())
            .to_string_lossy()
            .into_owned()
    } else {
        name
    };
    if item.temporary.is_none() {
        item.retain_temporary(source);
    }
    Ok(item)
}

pub async fn prepare_clipboard_image(
    data_uri: String,
    media: bool,
    compress: bool,
) -> MediaResult<PreparedAttachment> {
    crate::blocking(move || {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        const MAX_BYTES: usize = 48 * 1024 * 1024;
        let invalid = || {
            MediaError::InvalidAttachment("The clipboard image is invalid or exceeds 48 MiB".into())
        };
        if data_uri.len() > MAX_BYTES * 4 / 3 + 128 {
            return Err(invalid());
        }
        let (header, encoded) = data_uri.split_once(',').ok_or_else(invalid)?;
        let extension = match header {
            "data:image/png;base64" => "png",
            "data:image/jpeg;base64" => "jpg",
            "data:image/gif;base64" => "gif",
            "data:image/webp;base64" => "webp",
            _ => return Err(invalid()),
        };
        let bytes = STANDARD.decode(encoded).map_err(|_| invalid())?;
        if bytes.is_empty() || bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        let source = tempfile::Builder::new()
            .prefix("mixin-clipboard-")
            .suffix(&format!(".{extension}"))
            .tempfile()?;
        std::fs::write(source.path(), bytes)?;
        let mut item = prepare_file(source.path(), media, compress)?;
        if item.temporary.is_none() {
            item.temporary = Some(Arc::new(source.into_temp_path()));
        }
        item.name = format!(
            "image.{}",
            if item.mime_type == "image/jpeg" {
                "jpg"
            } else {
                extension
            }
        );
        Ok(item)
    })
    .await
    .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?
}

fn prepare_file(path: &Path, media: bool, compress: bool) -> MediaResult<PreparedAttachment> {
    let source = path.canonicalize()?;
    let metadata = source.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(MediaError::InvalidAttachment(
            "Choose a nonempty file".into(),
        ));
    }
    let name = source
        .file_name()
        .ok_or_else(|| MediaError::InvalidAttachment("File has no name".into()))?
        .to_string_lossy()
        .into_owned();
    let mime_type = mime_guess::from_path(&source)
        .first_or_octet_stream()
        .to_string();
    let mut item = PreparedAttachment {
        path: source.to_string_lossy().into_owned(),
        name,
        mime_type,
        kind: "DATA".into(),
        size: metadata.len(),
        ..Default::default()
    };
    if !media || !item.mime_type.starts_with("image/") {
        return Ok(item);
    }
    let mut reader = ImageReader::open(&source)?.with_guessed_format()?;
    let format = reader
        .format()
        .ok_or_else(|| MediaError::InvalidAttachment("Unknown image format".into()))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?;
    let orientation = decoder
        .orientation()
        .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?;
    let mut decoded = image::DynamicImage::from_decoder(decoder)
        .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?;
    decoded.apply_orientation(orientation);
    if decoded.width() == 0 || decoded.height() == 0 {
        return Err(MediaError::InvalidAttachment("Image is empty".into()));
    }
    // GIF animation must remain intact; a first-frame decode is only a thumbnail.
    let decoded = if compress
        && format != ImageFormat::Gif
        && (decoded.width() > 2048 || decoded.height() > 2048)
    {
        decoded.thumbnail(2048, 2048)
    } else {
        decoded
    };
    item.width = Some(decoded.width() as i32);
    item.height = Some(decoded.height() as i32);
    item.kind = "IMAGE".into();
    item.mime_type = format.to_mime_type().into();
    let mut thumbnail = Vec::new();
    JpegEncoder::new_with_quality(&mut thumbnail, 75)
        .encode_image(
            &decoded
                .thumbnail(320.min(decoded.width()), 320.min(decoded.height()))
                .to_rgb8(),
        )
        .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    item.thumbnail = Some(STANDARD.encode(thumbnail));
    if compress && format != ImageFormat::Gif {
        let (format, extension) = if decoded.color().has_alpha() {
            (ImageFormat::Png, "png")
        } else {
            (ImageFormat::Jpeg, "jpg")
        };
        let temporary = tempfile::Builder::new()
            .prefix("mixin-attachment-")
            .suffix(&format!(".{extension}"))
            .tempfile()?;
        let mut encoded = Cursor::new(Vec::new());
        if format == ImageFormat::Jpeg {
            JpegEncoder::new_with_quality(&mut encoded, 85)
                .encode_image(&decoded)
                .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?;
        } else {
            decoded
                .write_to(&mut encoded, format)
                .map_err(|error| MediaError::InvalidAttachment(error.to_string()))?;
        }
        std::fs::write(temporary.path(), encoded.into_inner())?;
        item.size = temporary.as_file().metadata()?.len();
        item.path = temporary.path().to_string_lossy().into_owned();
        item.name = PathBuf::from(&item.name)
            .with_extension(extension)
            .to_string_lossy()
            .into_owned();
        item.mime_type = format.to_mime_type().into();
        item.temporary = Some(Arc::new(temporary.into_temp_path()));
    }
    Ok(item)
}

#[cfg(target_os = "macos")]
async fn probe_video(path: &str) -> MediaResult<VideoInfo> {
    video_macos::probe(path).await
}

#[cfg(target_arch = "wasm32")]
async fn probe_video(_path: &str) -> MediaResult<VideoInfo> {
    Err(MediaError::Unsupported("video metadata reading"))
}

#[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
async fn probe_video(path: &str) -> MediaResult<VideoInfo> {
    // Packaged desktop apps cannot rely on their working directory or PATH
    // containing the probe shipped alongside the executable.
    let executable = std::env::var_os("MIXIN_FFPROBE")
        .or_else(|| {
            let executable = std::env::current_exe().ok()?;
            let probe = executable.parent()?.join(if cfg!(target_os = "windows") {
                "ffprobe.exe"
            } else {
                "ffprobe"
            });
            probe.is_file().then(|| probe.into_os_string())
        })
        .unwrap_or_else(|| "ffprobe".into());
    let mut command = tokio::process::Command::new(executable);
    command.args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height,duration:stream_tags=rotate:stream_side_data=rotation:format=duration", "-of", "json", "-i", path])
        .stdin(Stdio::null()).kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(15), command.output()).await
        .map_err(|_| MediaError::InvalidAttachment("Video metadata reading timed out".into()))?
        .map_err(|error| if error.kind() == std::io::ErrorKind::NotFound {
            MediaError::InvalidAttachment("Video preparation requires ffprobe; choose Send as file or configure MIXIN_FFPROBE".into())
        } else { error.into() })?;
    if !output.status.success() {
        return Err(MediaError::InvalidAttachment(
            "Could not read the video metadata".into(),
        ));
    }
    let (width, height, duration_millis) = parse_video_info(&output.stdout)?;
    Ok(VideoInfo {
        width,
        height,
        duration_millis,
        thumbnail: None,
    })
}

#[cfg(any(not(any(target_os = "macos", target_arch = "wasm32")), test))]
fn parse_video_info(bytes: &[u8]) -> MediaResult<(i32, i32, i64)> {
    let invalid =
        || MediaError::InvalidAttachment("Video dimensions or duration are invalid".into());
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let stream = value
        .get("streams")
        .and_then(|v| v.as_array())
        .and_then(|v| v.first())
        .ok_or_else(invalid)?;
    let mut width = stream
        .get("width")
        .and_then(|v| v.as_i64())
        .and_then(|v| i32::try_from(v).ok())
        .ok_or_else(invalid)?;
    let mut height = stream
        .get("height")
        .and_then(|v| v.as_i64())
        .and_then(|v| i32::try_from(v).ok())
        .ok_or_else(invalid)?;
    let seconds = stream
        .get("duration")
        .or_else(|| value.get("format").and_then(|v| v.get("duration")))
        .and_then(|v| {
            v.as_str()
                .and_then(|v| v.parse::<f64>().ok())
                .or_else(|| v.as_f64())
        })
        .ok_or_else(invalid)?;
    if width <= 0
        || height <= 0
        || !seconds.is_finite()
        || seconds <= 0.0
        || seconds * 1000.0 > i64::MAX as f64
    {
        return Err(invalid());
    }
    let rotation = stream
        .get("side_data_list")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter()
                .find_map(|v| v.get("rotation").and_then(|v| v.as_i64()))
        })
        .or_else(|| {
            stream
                .get("tags")
                .and_then(|v| v.get("rotate"))
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or_default();
    if rotation.rem_euclid(180) == 90 {
        std::mem::swap(&mut width, &mut height);
    }
    Ok((width, height, (seconds * 1000.0).round().max(1.0) as i64))
}

#[cfg(test)]
mod tests;
