//! Message input surfaces. Each dialog owns its editing state and cancellation.
mod attachments;
mod emoji;
mod files;
mod forms;
mod image_editor;
mod input;
mod mentions;
pub(crate) mod sticker_store;
pub(crate) mod stickers;
pub(crate) mod surface;
mod voice;

#[cfg(debug_assertions)]
pub(crate) use attachments::attachment_menu;
pub(crate) use attachments::attachments;
pub(crate) use files::{Tray, mount as mount_tray, preparation::local_sources};
#[cfg(debug_assertions)]
pub(crate) use files::{
    archive_password, attachment_frame, attachment_strip, file_content, original_toggle,
    remove_button, zip_toggle,
};
pub(crate) use input::Input;
pub(crate) use mentions::mentions;
pub(crate) use sticker_store::detail as sticker_detail;
pub(crate) use stickers::stickers;
pub(crate) use voice::Voice;
#[cfg(debug_assertions)]
pub(crate) use voice::preview_playback;
#[cfg(debug_assertions)]
pub(crate) use voice::surface::RecordingPanel;

use mixin_desktop_api::MessageListView;
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
    ui::icon_button()
        .size(ui::button_height_small())
        .min_h_0()
        .p_0()
        .rounded(ui::radius_md())
        .text_color(ui::fg_secondary)
}

/// A dialog with room for a document: the file preview and the post editor.
fn wide_dialog() -> View {
    ui::dialog_card(ui::DialogWidth::Wide)
}
