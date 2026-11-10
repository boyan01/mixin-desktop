//! Disposable avatar sources and bounded square export, independent of presentation.
use std::{
    io::{Cursor, Read},
    path::Path,
};

use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

use crate::{ImageEdit, MediaError, MediaResult, PreparedAttachment};

const SOURCE_BYTES: u64 = 10 * 1024 * 1024;
const PREPARED_BYTES: u64 = 128 * 1024 * 1024;

pub async fn prepare(path: String) -> MediaResult<PreparedAttachment> {
    crate::blocking(move || {
        let decoded = decode(Path::new(&path), SOURCE_BYTES)?;
        let temporary = tempfile::Builder::new()
            .prefix("mixin-avatar-")
            .suffix(".png")
            .tempfile()?;
        decoded
            .save_with_format(temporary.path(), ImageFormat::Png)
            .map_err(|error| invalid(error.to_string()))?;
        let mut source = PreparedAttachment::default();
        source.path = temporary.path().to_string_lossy().into_owned();
        source.name = "avatar.png".into();
        source.mime_type = "image/png".into();
        source.kind = "IMAGE".into();
        source.width = Some(decoded.width() as i32);
        source.height = Some(decoded.height() as i32);
        source.size = temporary.as_file().metadata()?.len();
        source.retain_temporary(temporary.into_temp_path());
        Ok(source)
    })
    .await
    .map_err(|error| invalid(error.to_string()))?
}

pub async fn encode(
    source: PreparedAttachment,
    crop: ImageEdit,
    group: bool,
) -> MediaResult<String> {
    crate::blocking(move || {
        // Keep the lease alive until decoding and export have completed.
        let decoded = decode(Path::new(&source.path), PREPARED_BYTES)?;
        let ImageEdit::Crop {
            x,
            y,
            width,
            height,
        } = crop
        else {
            return Err(invalid("An avatar requires a square crop"));
        };
        if width == 0
            || width != height
            || x.checked_add(width)
                .is_none_or(|edge| edge > decoded.width())
            || y.checked_add(height)
                .is_none_or(|edge| edge > decoded.height())
        {
            return Err(invalid("The square crop must stay inside the image"));
        }
        let side = width.min(512);
        let avatar = decoded.crop_imm(x, y, width, height).thumbnail(side, side);
        let mut output = Cursor::new(Vec::new());
        if group {
            DynamicImage::ImageRgb8(avatar.to_rgb8()).write_to(&mut output, ImageFormat::Jpeg)
        } else {
            avatar.write_to(&mut output, ImageFormat::Png)
        }
        .map_err(|error| invalid(error.to_string()))?;
        Ok(if group {
            URL_SAFE_NO_PAD.encode(output.into_inner())
        } else {
            STANDARD.encode(output.into_inner())
        })
    })
    .await
    .map_err(|error| invalid(error.to_string()))?
}

fn decode(path: &Path, maximum_bytes: u64) -> MediaResult<DynamicImage> {
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(maximum_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum_bytes {
        return Err(invalid("The avatar image exceeds the file size limit"));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(
            ImageFormat::Png
                | ImageFormat::Jpeg
                | ImageFormat::Gif
                | ImageFormat::WebP
                | ImageFormat::Bmp
        )
    ) {
        return Err(invalid("Choose a PNG, JPEG, GIF, WebP or BMP image"));
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| invalid(error.to_string()))?;
    let orientation = decoder
        .orientation()
        .map_err(|error| invalid(error.to_string()))?;
    let mut decoded =
        DynamicImage::from_decoder(decoder).map_err(|error| invalid(error.to_string()))?;
    decoded.apply_orientation(orientation);
    if decoded.width() == 0 || decoded.height() == 0 {
        return Err(invalid("The image is empty"));
    }
    Ok(decoded)
}

fn invalid(message: impl Into<String>) -> MediaError {
    MediaError::InvalidAttachment(message.into())
}

#[cfg(test)]
mod tests;
