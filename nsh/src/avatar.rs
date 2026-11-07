//! Shared avatar selection. Draft crop state belongs only to the owning page.
mod crop;

use crate::{motion::Presence, runtime};
use mixin_desktop_api::{ClientError, ClientResult, MediaClient, PreparedAttachment};
use nsh_core::{
    file_dialog::{FileFilter, OpenDialog},
    prelude::*,
};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(crate) struct Picker {
    source: Signal<Option<PreparedAttachment>>,
    picking: Signal<bool>,
    open: Signal<bool>,
    presence: Presence,
    media: Stored<Arc<MediaClient>>,
}

impl Picker {
    pub(crate) fn new() -> Self {
        let open = signal(false);
        let source = signal_with(None, SignalEquals::Never);
        let presence = Presence::modal(move || open.get());
        effect(move || {
            if !presence.mounted() && !open.get() {
                source.set(None);
            }
        });
        let media = use_context::<Stored<Option<Arc<MediaClient>>>>()
            .and_then(|media| media.with(|media| media.clone()))
            .expect("Account pages have the desktop media client");
        Self {
            source,
            picking: signal(false),
            open,
            presence,
            media: stored(media),
        }
    }

    pub(crate) fn busy(self) -> bool {
        self.picking.get() || self.presence.mounted() || self.open.get()
    }

    pub(crate) async fn pick(self) -> ClientResult<()> {
        if self.busy() {
            return Ok(());
        }
        self.picking.set(true);
        let result = self.select().await;
        self.picking.set(false);
        result
    }

    async fn select(self) -> ClientResult<()> {
        let paths = pick_paths(OpenDialog::files().filter(FileFilter::new(
            "Images",
            ["png", "jpg", "jpeg", "gif", "webp", "bmp"],
        )))
        .await
        .map_err(ClientError::Internal)?;
        let Some(path) = paths.and_then(|paths| paths.into_iter().next()) else {
            return Ok(());
        };
        let media = self.media.with(|media| media.clone());
        let source = runtime::run(async move {
            media
                .prepare_avatar(path.to_string_lossy().into_owned())
                .await
        })
        .await?;
        batch(|| {
            self.source.set(Some(source));
            self.open.set(true);
        });
        Ok(())
    }

    pub(crate) fn surface(
        self,
        group: bool,
        save: impl Fn(String) + Copy + 'static,
    ) -> impl IntoElement {
        view()
            .gap_2()
            .child(
                text(move || {
                    crate::i18n::tr(
                        "PNG, JPEG, GIF, WebP or BMP · up to 10 MB",
                        "PNG、JPEG、GIF、WebP 或 BMP，最大 10 MB",
                    )
                })
                .text_xs()
                .text_color(move || nsh_base::theme().muted_foreground),
            )
            .child(when(
                move || self.presence.mounted(),
                move || {
                    let source = self
                        .source
                        .with_untracked(|source| source.clone())
                        .expect("Open avatar picker has a prepared image");
                    crop::screen(self, source, group, save)
                },
            ))
    }
}
