//! Message input surfaces. Each dialog owns its editing state and cancellation.
mod attachments;
mod emoji;
mod files;
mod forms;
mod gifs;
mod image_editor;
mod input;
mod mentions;
mod sticker_store;
mod stickers;
mod voice;

pub(crate) use attachments::attachments;
pub(crate) use input::Input;
pub(crate) use mentions::mentions;
pub(crate) use sticker_store::detail as sticker_detail;
pub(crate) use stickers::stickers;
pub(crate) use voice::Voice;

use crate::i18n::I18n;

#[derive(Clone, PartialEq)]
pub(crate) enum AttachmentSource {
    Path(String),
    Clipboard {
        name: String,
        bytes: std::sync::Arc<Vec<u8>>,
    },
}

fn tr(i18n: I18n, en: &'static str, zh: &'static str) -> &'static str {
    i18n.tr(en, zh)
}
