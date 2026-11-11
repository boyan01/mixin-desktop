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

pub(super) const CARD_ACTIONS: &[&str] = &["Forward failed", "Reset"];
pub(super) fn cards(actions: Actions) -> View {
    let mode = super::button_pages::mode(actions, CARD_ACTIONS);
    provide_services();
    timeline_view(dynamic(move || {
        let rows = if mode.get() == "Forward failed" {
            fixtures::messages::retry_transcript_rows()
        } else {
            fixtures::messages::card_rows()
        };
        untracked(|| {
            chat::detached_messages(
                fixtures::USER_ID,
                rows,
                true,
                fixtures::messages::mention_names(),
            )
        })
    }))
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

pub(super) const SWITCH_ACTIONS: &[&str] = &["Switch"];

/// How long a mock chat takes to read its first rows: longer than a local read,
/// so that 100 ms capture frames show what the pane draws meanwhile.
const MOCK_READ_MS: u64 = 300;
/// When the list learns that the opened chat was read, as after its refresh.
const MOCK_REFRESH_MS: u64 = 450;

/// The conversation list beside the detail pane, as `Home` lays them out.
/// "Switch" opens the second, unread conversation from the first: the list
/// should change only its selection and that row's badge, and the pane should
/// never show a chat without its messages.
pub(super) fn switch(actions: Actions) -> View {
    provide_services();
    let selected = signal_with(None, SignalEquals::Never);
    let chat_navigation = navigation::ChatNavigation::new(selected, signal(None), signal(false));
    provide_context(chat_navigation);
    let fresh = move || {
        let mut rows = fixtures::conversations::stress_rows();
        rows.truncate(12);
        rows[0].unseen_count = 0;
        rows[1].unseen_count = 3;
        rows
    };
    let rows = signal(fresh());
    chat_navigation.select(rows.with_untracked(|rows| rows[0].clone()));
    actions.on(
        "Switch",
        move || {
            rows.set(fresh());
            chat_navigation.select(rows.with_untracked(|rows| rows[0].clone()));
        },
        move || {
            chat_navigation.select(rows.with_untracked(|rows| rows[1].clone()));
            set_timeout(
                std::time::Duration::from_millis(MOCK_REFRESH_MS),
                move || {
                    rows.update(|rows| rows[1].unseen_count = 0);
                },
            );
        },
    );
    view()
        .flex_1()
        .flex_row()
        .bg(ui::surface_pane)
        .child(
            view()
                .w(ui::sidebar_width)
                .flex_shrink_0()
                .bg(ui::surface_list)
                .child(crate::conversation_list::detached(
                    fixtures::USER_ID,
                    rows,
                    selected,
                    element_ref(),
                )),
        )
        .child(view().flex_1().min_w_0().min_h_0().child(chat::panes(
            selected,
            chat_navigation.selected_id,
            || view().into_element(),
            |row, ready| {
                let loaded = signal(false);
                set_timeout(std::time::Duration::from_millis(MOCK_READ_MS), move || {
                    loaded.set(true);
                    ready();
                });
                timeline_view(chat::preview_chrome(
                    row,
                    signal(None),
                    view().flex_1().min_h_0().child(when(loaded, || {
                        chat::detached_messages(
                            fixtures::USER_ID,
                            fixtures::messages::reply_rows(),
                            true,
                            fixtures::messages::mention_names(),
                        )
                    })),
                ))
            },
        )))
}

pub(super) const CHROME_ACTIONS: &[&str] = &["Long pin", "No pin", "Reset"];

/// The glass chrome over a timeline. The pin joins the title row from
/// `glass-inline-pin-width`; narrow the viewport to see it take its own row.
pub(super) fn chrome(actions: Actions) -> View {
    let mode = super::button_pages::mode(actions, CHROME_ACTIONS);
    provide_services();
    let pin = signal(None::<String>);
    effect(move || {
        pin.set(match mode.get() {
            "No pin" => None,
            "Long pin" => Some(
                "Ava pinned \"The release moves to Friday at 5 pm. Please finish the \
                 review before Thursday noon and post the notes in this group.\""
                    .into(),
            ),
            _ => Some("Ava pinned \"Release at 5 pm\"".into()),
        })
    });
    let row = fixtures::conversations::stress_rows()
        .into_iter()
        .find(|row| row.category == "GROUP")
        .expect("the stress rows include a group");
    timeline_view(chat::preview_chrome(
        row,
        pin,
        chat::detached_messages(
            fixtures::USER_ID,
            fixtures::messages::reply_rows(),
            true,
            fixtures::messages::mention_names(),
        ),
    ))
}

/// The chat background and its list, under the services `Home` provides a chat.
fn timeline(rows: Vec<MessageListView>) -> View {
    provide_services();
    timeline_view(chat::detached_messages(
        fixtures::USER_ID,
        rows,
        true,
        fixtures::messages::mention_names(),
    ))
}

/// The chat background under `list`, built after `provide_services`.
pub(super) fn timeline_view(list: impl IntoElement) -> View {
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
        .child(list)
}

/// What `Home` provides a chat, without an account: defaults and idle state.
pub(super) fn provide_services() {
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
