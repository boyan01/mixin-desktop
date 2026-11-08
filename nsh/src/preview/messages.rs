//! Production message rows over fixed messages, in a chat's own list. Each scene
//! holds a few related message kinds, few enough to fit the default capture
//! window; `fixtures::messages` lists the rows.
//!
//! Rows have no account: a press that needs one, such as a download or a menu
//! command, does nothing useful here.

use std::sync::Arc;

use mixin_desktop_api::{MediaClient, MessageListView};
use nsh_core::prelude::*;

use super::{Actions, fixtures};
use crate::{chat, navigation, ui};

pub(super) const ACTIONS: &[&str] = &[];

pub(super) fn text(_: Actions) -> View {
    timeline(fixtures::messages::text_rows())
}

pub(super) fn replies(_: Actions) -> View {
    timeline(fixtures::messages::reply_rows())
}

pub(super) fn photos(_: Actions) -> View {
    timeline(fixtures::messages::photo_rows())
}

pub(super) fn media(_: Actions) -> View {
    timeline(fixtures::messages::media_rows())
}

pub(super) fn files(_: Actions) -> View {
    timeline(fixtures::messages::file_rows())
}

pub(super) fn cards(_: Actions) -> View {
    timeline(fixtures::messages::card_rows())
}

pub(super) fn apps(_: Actions) -> View {
    timeline(fixtures::messages::app_rows())
}

pub(super) fn wallet(_: Actions) -> View {
    timeline(fixtures::messages::wallet_rows())
}

pub(super) fn system(_: Actions) -> View {
    timeline(fixtures::messages::system_rows())
}

/// The chat background and its list, under the services `Home` provides a chat.
fn timeline(rows: Vec<MessageListView>) -> View {
    provide_services();
    view()
        .flex_1()
        .min_h_0()
        .relative()
        .bg(ui::surface_chat)
        .child(
            ui::wallpaper()
                .absolute()
                .inset_0()
                .size_full()
                .object_fit(ResizeMode::Cover)
                .opacity(ui::chat_pattern_opacity()),
        )
        .child(chat::detached_messages(
            fixtures::USER_ID,
            rows,
            true,
            fixtures::messages::mention_names(),
        ))
}

/// What `Home` provides a chat, without an account: defaults and idle state.
fn provide_services() {
    let media = Arc::new(MediaClient::new());
    provide_context(crate::Clients {
        settings: stored(None),
        media: stored(Some(media.clone())),
    });
    provide_context(crate::appearance::Appearance::detached());
    provide_context(crate::attachment_progress::AttachmentProgress::detached());
    provide_context(chat::reedit::Reedit::mount());
    provide_context(ui::Toasts::mount());
    let nav = navigation::Navigation::new();
    provide_context(nav);
    provide_context(navigation::ChatNavigation::new(
        signal_with(None, SignalEquals::Never),
        signal_with(None, SignalEquals::Never),
        nav.settings,
    ));
    // Link routing reads chat navigation, and bot windows read link routing.
    provide_context(crate::links::LinkNavigation::detached());
    provide_context(crate::bot::detached());
    provide_context(crate::playback::Playback::detached(media));
}
