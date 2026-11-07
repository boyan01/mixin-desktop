//! Nondestructive raster edits. Coordinates are pixels in the current image.
use std::path::Path;

use image::{imageops, ImageDecoder, ImageFormat, ImageReader, Pixel, Rgba, RgbaImage};
use tempfile::TempPath;

use crate::{attachment, MediaError, MediaResult, PreparedAttachment};

/// An ordered image-edit recipe. Erasing affects annotations, never the source pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageEdit {
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    RotateClockwise,
    RotateCounterClockwise,
    FlipHorizontal,
    Stroke {
        points: Vec<(u32, u32)>,
        color: [u8; 4],
        width: u32,
    },
    EraseStroke {
        points: Vec<(u32, u32)>,
        width: u32,
    },
}

pub async fn apply(path: String, edits: Vec<ImageEdit>) -> MediaResult<PreparedAttachment> {
    let name = Path::new(&path)
        .file_name()
        .map(|name| {
            Path::new(name)
                .with_extension("png")
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "image.png".into());
    let temporary = tokio::task::spawn_blocking(move || render(Path::new(&path), &edits))
        .await
        .map_err(|error| invalid(error.to_string()))??;
    let mut prepared =
        attachment::prepare(temporary.to_string_lossy().into_owned(), true, false).await?;
    prepared.name = name;
    prepared.retain_temporary(temporary);
    Ok(prepared)
}

fn invalid(message: impl Into<String>) -> MediaError {
    MediaError::InvalidAttachment(message.into())
}

fn render(path: &Path, edits: &[ImageEdit]) -> MediaResult<TempPath> {
    if edits.len() > 4096 {
        return Err(invalid("The image has too many edits"));
    }
    let mut reader = ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| invalid(error.to_string()))?;
    let orientation = decoder
        .orientation()
        .map_err(|error| invalid(error.to_string()))?;
    let mut decoded =
        image::DynamicImage::from_decoder(decoder).map_err(|error| invalid(error.to_string()))?;
    decoded.apply_orientation(orientation);
    let mut source = decoded.into_rgba8();
    if source.width() == 0 || source.height() == 0 {
        return Err(invalid("The image is empty"));
    }
    let mut annotations = RgbaImage::new(source.width(), source.height());
    let mut work = 0_u64;
    for edit in edits {
        match edit {
            ImageEdit::Crop {
                x,
                y,
                width,
                height,
            } => {
                if *width == 0
                    || *height == 0
                    || x.checked_add(*width)
                        .is_none_or(|edge| edge > source.width())
                    || y.checked_add(*height)
                        .is_none_or(|edge| edge > source.height())
                {
                    return Err(invalid("The crop must stay inside the image"));
                }
                source = imageops::crop_imm(&source, *x, *y, *width, *height).to_image();
                annotations = imageops::crop_imm(&annotations, *x, *y, *width, *height).to_image();
            }
            ImageEdit::RotateClockwise => {
                source = imageops::rotate90(&source);
                annotations = imageops::rotate90(&annotations);
            }
            ImageEdit::RotateCounterClockwise => {
                source = imageops::rotate270(&source);
                annotations = imageops::rotate270(&annotations);
            }
            ImageEdit::FlipHorizontal => {
                source = imageops::flip_horizontal(&source);
                annotations = imageops::flip_horizontal(&annotations);
            }
            ImageEdit::Stroke {
                points,
                color,
                width,
            } => draw(&mut annotations, points, *width, Some(*color), &mut work)?,
            ImageEdit::EraseStroke { points, width } => {
                draw(&mut annotations, points, *width, None, &mut work)?
            }
        }
    }
    imageops::overlay(&mut source, &annotations, 0, 0);
    let temporary = tempfile::Builder::new()
        .prefix("mixin-edited-")
        .suffix(".png")
        .tempfile()?;
    source
        .save_with_format(temporary.path(), ImageFormat::Png)
        .map_err(|error| invalid(error.to_string()))?;
    Ok(temporary.into_temp_path())
}

fn draw(
    layer: &mut RgbaImage,
    points: &[(u32, u32)],
    width: u32,
    color: Option<[u8; 4]>,
    work: &mut u64,
) -> MediaResult<()> {
    if points.is_empty()
        || points.len() > 65536
        || width == 0
        || width > 1024
        || points
            .iter()
            .any(|(x, y)| *x >= layer.width() || *y >= layer.height())
    {
        return Err(invalid("The annotation has invalid points or brush width"));
    }
    // A stroke's coverage is unioned before compositing, so translucent joins
    // do not get darker and the eraser removes paint rather than original pixels.
    let mut mask = image::GrayImage::new(layer.width(), layer.height());
    let radius = width as f64 / 2.0;
    for index in 0..points.len().saturating_sub(1).max(1) {
        let from = points[index];
        let to = points.get(index + 1).copied().unwrap_or(from);
        let min_x = (f64::from(from.0.min(to.0)) - radius - 1.0).max(0.0) as u32;
        let min_y = (f64::from(from.1.min(to.1)) - radius - 1.0).max(0.0) as u32;
        let max_x =
            (f64::from(from.0.max(to.0)) + radius + 1.0).min(f64::from(layer.width() - 1)) as u32;
        let max_y =
            (f64::from(from.1.max(to.1)) + radius + 1.0).min(f64::from(layer.height() - 1)) as u32;
        *work = work.saturating_add(u64::from(max_x - min_x + 1) * u64::from(max_y - min_y + 1));
        if *work > 128 * 1024 * 1024 {
            return Err(invalid(
                "The image edit is too complex; undo some annotations and try again",
            ));
        }
        let (dx, dy) = (
            f64::from(to.0) - f64::from(from.0),
            f64::from(to.1) - f64::from(from.1),
        );
        let length_squared = dx * dx + dy * dy;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (px, py) = (
                    f64::from(x) - f64::from(from.0),
                    f64::from(y) - f64::from(from.1),
                );
                let fraction = if length_squared == 0.0 {
                    0.0
                } else {
                    ((px * dx + py * dy) / length_squared).clamp(0.0, 1.0)
                };
                let distance = ((px - fraction * dx).powi(2) + (py - fraction * dy).powi(2)).sqrt();
                let alpha = ((radius + 0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8;
                let pixel = mask.get_pixel_mut(x, y);
                pixel.0[0] = pixel.0[0].max(alpha);
            }
        }
    }
    for (pixel, coverage) in layer.pixels_mut().zip(mask.pixels()) {
        let coverage = u16::from(coverage.0[0]);
        if coverage == 0 {
            continue;
        }
        if let Some(mut color) = color {
            color[3] = (u16::from(color[3]) * coverage / 255) as u8;
            pixel.blend(&Rgba(color));
        } else {
            pixel.0[3] = (u16::from(pixel.0[3]) * (255 - coverage) / 255) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
