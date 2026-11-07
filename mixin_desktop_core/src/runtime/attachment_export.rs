use std::{
    io::Cursor,
    path::{Component, Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
use base64ct::{Base64, Encoding};
use sdk::message_category::MessageCategory as _;

use crate::{
    core::{
        attachment::{attachment_path, transcript_attachment_path},
        message::decrypt::transcript_attachment_message,
    },
    db::mixin::{message::MediaStatus, MixinDatabase},
};

pub(super) const MAX_IMAGE_BYTES: u64 = 48 << 20;
pub(super) const MAX_IMAGE_PIXELS: u64 = 32_000_000;

/// Resolve from persisted state, never from a presentation DTO's local URL.
pub(super) async fn source(
    database: &MixinDatabase,
    directory: &Path,
    message_id: &str,
    transcript_id: Option<&str>,
) -> Result<(PathBuf, bool)> {
    let message = if let Some(root) = transcript_id {
        let parent = database
            .message_dao
            .find_message_by_id(&root.to_owned())
            .await?
            .filter(|parent| parent.category.is_transcript())
            .ok_or_else(|| anyhow!("transcript not found: {root}"))?;
        let child = database
            .transcript_message_dao
            .find(root, message_id)
            .await?
            .ok_or_else(|| anyhow!("transcript message not found: {message_id}"))?;
        let mut message = transcript_attachment_message(&parent.conversation_id, &child)?;
        message.media_status = child.media_status.unwrap_or_default();
        message
    } else {
        database
            .message_dao
            .find_message_by_id(&message_id.to_owned())
            .await?
            .ok_or_else(|| anyhow!("message not found: {message_id}"))?
    };
    if !message.category.is_attachment() {
        bail!("message is not an attachment: {message_id}");
    }
    if !matches!(message.media_status, MediaStatus::Done | MediaStatus::Read) {
        bail!("attachment is not downloaded: {message_id}");
    }
    for value in [&message.message_id, &message.conversation_id] {
        let mut components = Path::new(value).components();
        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
            bail!("invalid attachment path component");
        }
    }
    let mut path = if transcript_id.is_some() {
        transcript_attachment_path(directory, &message)?
    } else {
        attachment_path(directory, &message)?
    };
    if let Some(root) = transcript_id {
        let mut components = Path::new(root).components();
        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
            bail!("invalid transcript path component");
        }
        if let Some(stored) = message
            .media_url
            .as_deref()
            .map(Path::new)
            .filter(|path| path.is_absolute())
        {
            let scoped = path
                .parent()
                .unwrap()
                .join(root)
                .join(path.file_name().unwrap());
            if stored != path && stored != scoped {
                bail!("transcript attachment source is outside its root");
            }
            path = stored.to_path_buf();
        }
    }
    let account = crate::fs::canonicalize(directory).await?;
    let path = crate::fs::canonicalize(path).await?;
    if !path.starts_with(account.join("Media")) || !crate::fs::metadata(&path).await?.is_file() {
        bail!("attachment source is outside the account media directory");
    }
    if let Some(root) = transcript_id {
        let transcript_directory = account.join("Media/Transcripts");
        if path.parent() != Some(transcript_directory.as_path())
            && path.parent() != Some(transcript_directory.join(root).as_path())
        {
            bail!("transcript attachment source is outside its root");
        }
    }
    Ok((path, message.category.is_image()))
}

pub(super) async fn save(source: &Path, destination: &Path) -> Result<()> {
    if crate::fs::canonicalize(destination)
        .await
        .is_ok_and(|path| path == source)
    {
        bail!("attachment destination is the source file");
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary = parent.join(format!(".mixin-export-{}", uuid::Uuid::new_v4()));
    // Reserve the temporary path before copying; replacing the destination by rename
    // also preserves the source if the selected destination is its hard link.
    crate::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .await?;
    let result = async {
        crate::fs::copy(source, &temporary).await?;
        crate::fs::rename(&temporary, destination).await?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = crate::fs::remove_file(&temporary).await;
    }
    result
}

pub(super) fn image_data_uri(bytes: Vec<u8>) -> Result<String> {
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        bail!("image exceeds the clipboard size limit");
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16_384);
    limits.max_image_height = Some(16_384);
    limits.max_alloc = Some(MAX_IMAGE_PIXELS * 8);
    reader.limits(limits);
    let decoder = reader.into_decoder()?;
    use image::ImageDecoder as _;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        bail!("image exceeds the clipboard pixel limit");
    }
    let image = image::DynamicImage::from_decoder(decoder)?;
    let mut png = Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png)?;
    let png = png.into_inner();
    if png.len() as u64 > MAX_IMAGE_BYTES {
        bail!("image exceeds the clipboard size limit");
    }
    Ok(format!(
        "data:image/png;base64,{}",
        Base64::encode_string(&png)
    ))
}

#[cfg(test)]
#[path = "attachment_export_tests.rs"]
mod tests;
