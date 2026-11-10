//! Retains prepared sources across appends; each source has one current recipe.
use super::Mode;
use crate::{composer::AttachmentSource, runtime};
use mixin_desktop_api::{ClientError, ClientResult, ImageEdit, MediaClient, PreparedAttachment};
use nsh_core::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub(super) struct Source {
    pub id: u64,
    pub input: AttachmentSource,
    pub original: Option<PreparedAttachment>,
    pub edits: Vec<ImageEdit>,
    pub edited: Option<PreparedAttachment>,
}

impl Source {
    pub fn new(id: u64, input: AttachmentSource) -> Self {
        Self {
            id,
            input,
            original: None,
            edits: Vec::new(),
            edited: None,
        }
    }

    pub fn name(&self) -> String {
        match &self.input {
            AttachmentSource::Path(path) => std::path::Path::new(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            AttachmentSource::Clipboard { name, .. } => name.clone(),
        }
    }
}

#[derive(Clone)]
struct Cached {
    media: bool,
    compress: bool,
    edited: Option<String>,
    item: PreparedAttachment,
    preview: PreparedAttachment,
}

/// Rows survive a cancelled preparation, so adding a file or changing the mode
/// never repeats compression of the rows that already finished.
#[derive(Clone, Default)]
pub(super) struct Cache(Arc<Mutex<HashMap<u64, Cached>>>);

impl Cache {
    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<u64, Cached>> {
        self.0.lock().expect("attachment cache")
    }

    pub fn item(&self, id: u64) -> Option<PreparedAttachment> {
        self.entries().get(&id).map(|entry| entry.preview.clone())
    }

    /// Forgets removed rows and returns the finished items in source order.
    pub fn retain(&self, sources: &[Source]) -> Vec<PreparedAttachment> {
        let mut entries = self.entries();
        entries.retain(|id, _| sources.iter().any(|source| source.id == *id));
        sources
            .iter()
            .filter_map(|source| entries.get(&source.id).map(|entry| entry.item.clone()))
            .collect()
    }
}

pub(super) async fn prepare_source(
    media: &MediaClient,
    source: &AttachmentSource,
    as_media: bool,
    compress: bool,
) -> ClientResult<PreparedAttachment> {
    match source {
        AttachmentSource::Path(path) => {
            media
                .prepare_attachment(path.clone(), as_media, compress)
                .await
        }
        AttachmentSource::Clipboard { name, bytes } => {
            media
                .prepare_clipboard_file(name.clone(), bytes.as_ref().clone(), as_media, compress)
                .await
        }
    }
}

/// A failure names its file: the rest of the batch stays usable once it is removed.
pub(super) async fn prepare_sources(
    media: Arc<MediaClient>,
    sources: Vec<Source>,
    mode: Mode,
    compress: bool,
    cache: Cache,
) -> ClientResult<Vec<PreparedAttachment>> {
    let as_media = matches!(mode, Mode::Auto | Mode::Media);
    let compress = as_media && compress;
    let mut items = Vec::with_capacity(sources.len());
    for source in &sources {
        let edited = source.edited.as_ref().map(|item| item.path.clone());
        let current = cache.entries().get(&source.id).and_then(|entry| {
            (entry.media == as_media && entry.compress == compress && entry.edited == edited)
                .then(|| entry.item.clone())
        });
        if let Some(item) = current {
            items.push(item);
            continue;
        }
        let prepared = if let Some(retained) = &source.edited {
            media
                .prepare_attachment(retained.path.clone(), as_media, compress)
                .await
                .map(|mut item| {
                    item.name = retained.name.clone();
                    item
                })
        } else {
            prepare_source(&media, &source.input, as_media, compress).await
        };
        let item = prepared
            .map_err(|error| ClientError::Internal(format!("{}: {error}", source.name())))?;
        // ZIP changes the send recipe, not the visible identity of each source.
        let preview = cache
            .entries()
            .get(&source.id)
            .and_then(|previous| {
                (previous.edited == edited
                    && (mode == Mode::Archive
                        || (previous.preview.kind == "IMAGE" && item.kind == "IMAGE")))
                    .then(|| previous.preview.clone())
            })
            .unwrap_or_else(|| item.clone());
        cache.entries().insert(
            source.id,
            Cached {
                media: as_media,
                compress,
                edited,
                item: item.clone(),
                preview,
            },
        );
        items.push(item);
    }
    Ok(items)
}

/// The local paths of dropped or pasted URIs.
pub(crate) fn local_sources(uris: &[String]) -> Vec<AttachmentSource> {
    uris.iter()
        .filter_map(|uri| file_path(uri))
        .map(AttachmentSource::Path)
        .collect()
}

/// Finder hands over file references (`file:///.file/id=…`), which only Foundation can
/// resolve to a path.
#[cfg(target_os = "macos")]
fn file_path(uri: &str) -> Option<String> {
    use objc2_foundation::{NSString, NSURL};
    let url = NSURL::URLWithString(&NSString::from_str(uri))?;
    if !url.isFileURL() {
        return None;
    }
    let resolved = url.filePathURL()?;
    Some(resolved.path()?.to_string())
}

// Browser clipboard URIs do not name local files.
#[cfg(target_arch = "wasm32")]
fn file_path(_uri: &str) -> Option<String> {
    None
}

#[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
fn file_path(uri: &str) -> Option<String> {
    let path = url::Url::parse(uri).ok()?.to_file_path().ok()?;
    Some(path.to_string_lossy().into_owned())
}

pub(super) async fn clipboard_sources() -> Vec<AttachmentSource> {
    let paths = local_sources(&clipboard_files().await);
    if !paths.is_empty() {
        return paths;
    }
    let Some(image) = clipboard_image().await else {
        return Vec::new();
    };
    // A screenshot's data URI is megabytes: decode it off the engine thread.
    let bytes = runtime::run(async move { Ok(decode_image(&image.url)) }).await;
    let Ok(Some(bytes)) = bytes else {
        return Vec::new();
    };
    vec![AttachmentSource::Clipboard {
        name: "Image.png".into(),
        bytes: Arc::new(bytes),
    }]
}

fn decode_image(url: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let (header, bytes) = url.split_once(',')?;
    if !header.starts_with("data:image/") || !header.ends_with(";base64") {
        return None;
    }
    base64::engine::general_purpose::STANDARD.decode(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn packaging_images_preserves_their_preview_and_archives_original_bytes() {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(32, 32, image::Rgb([45, 120, 210]))
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let bytes = bytes.into_inner();
        let source = Source::new(
            0,
            AttachmentSource::Clipboard {
                name: "fixture.png".into(),
                bytes: Arc::new(bytes.clone()),
            },
        );
        let media = Arc::new(MediaClient::new());
        let cache = Cache::default();
        let pictures = prepare_sources(
            media.clone(),
            vec![source.clone()],
            Mode::Media,
            true,
            cache.clone(),
        )
        .await
        .unwrap();
        let preview = cache.item(0).unwrap();
        assert_eq!(preview.kind, "IMAGE");
        assert_eq!(preview.path, pictures[0].path);
        let archive = prepare_sources(
            media.clone(),
            vec![source.clone()],
            Mode::Archive,
            true,
            cache.clone(),
        )
        .await
        .unwrap();
        assert_eq!(archive[0].kind, "DATA");
        assert_eq!(std::fs::read(&archive[0].path).unwrap(), bytes);
        assert_eq!(cache.item(0).unwrap().kind, "IMAGE");
        assert_eq!(cache.item(0).unwrap().path, preview.path);
        prepare_sources(
            media.clone(),
            vec![source],
            Mode::Media,
            true,
            cache.clone(),
        )
        .await
        .unwrap();
        assert_eq!(cache.item(0).unwrap().kind, "IMAGE");
        assert_eq!(cache.item(0).unwrap().path, preview.path);
        media.close().await.unwrap();
    }
}
