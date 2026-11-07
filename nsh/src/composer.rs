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

use std::sync::Arc;

use mixin_desktop_api::{MediaClient, MessageListView};
use nsh_base::ButtonElement;
use nsh_core::prelude::*;

use crate::{i18n::I18n, ui};

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

fn media_client() -> Arc<MediaClient> {
    use_context::<Stored<Option<Arc<MediaClient>>>>()
        .expect("App provides media")
        .with(|media| media.clone())
        .expect("Desktop media initialized")
}

fn quote_id(quote: Signal<Option<MessageListView>>) -> Option<String> {
    quote.with_untracked(|quote| quote.as_ref().map(|message| message.message_id.clone()))
}

/// Clears the reply only when the user has not chosen another during the send.
fn clear_quote(quote: Signal<Option<MessageListView>>, sent: Option<String>) {
    if quote_id(quote) == sent {
        quote.set(None);
    }
}

fn tool_button() -> ButtonElement {
    ui::round_button().size(ui::field_height())
}

/// A dialog with room for a document: the file preview and the post editor.
fn wide_dialog() -> View {
    ui::dialog_card(ui::DialogWidth::Wide)
}
