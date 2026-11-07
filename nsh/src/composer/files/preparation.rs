//! Retains prepared sources across appends; each source has one current recipe.
use super::Mode;
use crate::composer::AttachmentSource;
use mixin_desktop_api::{ClientResult, ImageEdit, MediaClient, PreparedAttachment};
use nsh_core::prelude::*;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(super) struct Source {
    pub id: u64,
    pub input: AttachmentSource,
    pub original: Option<PreparedAttachment>,
    pub edits: Vec<ImageEdit>,
    pub edited: Option<PreparedAttachment>,
}

#[derive(Clone)]
pub(super) struct Cached {
    media: bool,
    compress: bool,
    edited: Option<String>,
    pub item: PreparedAttachment,
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

pub(super) async fn prepare_sources(
    media: Arc<MediaClient>,
    sources: Vec<Source>,
    mode: Mode,
    compress: bool,
    cached: HashMap<u64, Cached>,
) -> ClientResult<(Vec<PreparedAttachment>, HashMap<u64, Cached>)> {
    let as_media = matches!(mode, Mode::Auto | Mode::Media);
    let compress = as_media && compress;
    let mut items = Vec::with_capacity(sources.len());
    let mut cache = HashMap::with_capacity(sources.len());
    for source in &sources {
        let edited = source.edited.as_ref().map(|item| item.path.clone());
        let item = if let Some(entry) = cached.get(&source.id).filter(|entry| {
            entry.media == as_media && entry.compress == compress && entry.edited == edited
        }) {
            entry.item.clone()
        } else if let Some(retained) = &source.edited {
            let mut item = media
                .prepare_attachment(retained.path.clone(), as_media, compress)
                .await?;
            item.name = retained.name.clone();
            item
        } else {
            prepare_source(&media, &source.input, as_media, compress).await?
        };
        items.push(item.clone());
        cache.insert(
            source.id,
            Cached {
                media: as_media,
                compress,
                edited,
                item,
            },
        );
    }
    Ok((items, cache))
}

pub(super) fn local_sources(uris: &[String]) -> Vec<AttachmentSource> {
    uris.iter()
        .filter_map(|uri| url::Url::parse(uri).ok()?.to_file_path().ok())
        .map(|path| AttachmentSource::Path(path.to_string_lossy().into_owned()))
        .collect()
}

pub(super) fn pasted_sources(event: &nsh_core::PasteEvent) -> Vec<AttachmentSource> {
    let paths = local_sources(&event.uris);
    if !paths.is_empty() {
        paths
    } else {
        event
            .files
            .iter()
            .map(|file| AttachmentSource::Clipboard {
                name: file.name.clone(),
                bytes: Arc::new(file.data.clone()),
            })
            .collect()
    }
}

pub(super) async fn clipboard_sources() -> Vec<AttachmentSource> {
    let paths = local_sources(&clipboard_files().await);
    if !paths.is_empty() {
        return paths;
    }
    let Some(image) = clipboard_image().await else {
        return Vec::new();
    };
    use base64::Engine as _;
    let Some((header, bytes)) = image.url.split_once(',') else {
        return Vec::new();
    };
    if !header.starts_with("data:image/") || !header.ends_with(";base64") {
        return Vec::new();
    }
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(bytes) else {
        return Vec::new();
    };
    vec![AttachmentSource::Clipboard {
        name: "Image.png".into(),
        bytes: Arc::new(bytes),
    }]
}

pub(super) fn name(source: &Source) -> String {
    match &source.input {
        AttachmentSource::Path(path) => std::path::Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        AttachmentSource::Clipboard { name, .. } => name.clone(),
    }
}
