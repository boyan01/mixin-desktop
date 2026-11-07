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
use nsh_base::{ButtonElement, menu_item, theme};
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
    ui::icon_button().size_10().px_1().py_1()
}

fn menu_row(
    label: impl Fn() -> &'static str + 'static,
    select: impl Fn() + 'static,
) -> impl IntoElement {
    menu_item()
        .px_3()
        .py_2()
        .rounded_md()
        .hover(|s| s.bg(move || theme().accent))
        .child(text(label).text_sm())
        .on_select(select)
}

fn failure(label: impl Fn() -> &'static str + 'static) -> impl IntoElement {
    text(label)
        .text_sm()
        .text_color(move || theme().destructive)
}

/// The card every composer dialog presents inside `ui::modal`.
fn dialog_card() -> View {
    view()
        .max_w(relative(1.0))
        .max_h(relative(0.9))
        .min_h_0()
        .p_6()
        .gap_3()
        .rounded_xl()
        .bg(move || theme().popover)
        .text_color(move || theme().popover_foreground)
}

fn dialog_title(label: impl Fn() -> String + 'static) -> impl IntoElement {
    text(label)
        .flex_none()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
}
