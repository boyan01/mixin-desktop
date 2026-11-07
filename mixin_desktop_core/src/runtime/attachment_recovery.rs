//! Revalidate completed photos before retrying their persisted download.
use std::{io::Cursor, path::Path};

use anyhow::{bail, Result};
use image::ImageDecoder as _;

use crate::db::{mixin::message::MediaStatus, MixinDatabase};

pub(super) async fn download_required(
    database: &MixinDatabase,
    directory: &Path,
    message_id: &str,
    transcript: Option<&str>,
    category: &str,
    status: MediaStatus,
) -> Result<bool> {
    if status == MediaStatus::Canceled {
        return Ok(true);
    }
    if !category.ends_with("_IMAGE") || !matches!(status, MediaStatus::Done | MediaStatus::Read) {
        bail!("attachment is not downloadable: {message_id}");
    }
    let path = match super::export::source(database, directory, message_id, transcript).await {
        Ok((path, _)) => path,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(true)
        }
        Err(error) => return Err(error),
    };
    let metadata = crate::fs::metadata(&path).await?;
    if metadata.len() > super::export::MAX_IMAGE_BYTES {
        bail!("image exceeds the supported preview size");
    }
    let bytes = match crate::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error.into()),
    };
    if bytes.len() as u64 > super::export::MAX_IMAGE_BYTES {
        bail!("image exceeds the supported preview size");
    }
    tokio::task::spawn_blocking(move || corrupt_image(bytes)).await?
}

fn corrupt_image(bytes: Vec<u8>) -> Result<bool> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    if reader.format().is_none() {
        return Ok(true);
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(super::export::MAX_IMAGE_PIXELS * 8);
    reader.limits(limits);
    let result = (|| -> Result<()> {
        let decoder = reader.into_decoder()?;
        let (width, height) = decoder.dimensions();
        if u64::from(width) * u64::from(height) > super::export::MAX_IMAGE_PIXELS {
            bail!("image exceeds the supported preview dimensions");
        }
        image::DynamicImage::from_decoder(decoder)?;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(false),
        Err(error)
            if error
                .downcast_ref::<image::ImageError>()
                .is_some_and(|error| {
                    matches!(error, image::ImageError::Decoding(_))
                        || matches!(error, image::ImageError::IoError(error)
                    if error.kind() == std::io::ErrorKind::UnexpectedEof)
                }) =>
        {
            Ok(true)
        }
        // A valid but unsupported/oversized format cannot be repaired by
        // fetching identical bytes. Preserve it instead of replacing it.
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "attachment_recovery_tests.rs"]
mod tests;
