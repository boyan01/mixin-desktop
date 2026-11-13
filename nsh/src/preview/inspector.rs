//! The production floating inspector over chat chrome and fixed message rows.
use nsh_core::prelude::*;

use super::{Actions, fixtures, messages};
use crate::{chat, inspector, navigation::Panel};

pub(super) const ACTIONS: &[&str] = &["Open info", "Open search", "Open pinned", "Close"];

pub(super) fn scene(actions: Actions) -> View {
    messages::provide_services();
    let panel = signal(None);
    for (action, destination) in [
        ("Open info", Panel::Info),
        ("Open search", Panel::Search),
        ("Open pinned", Panel::Pinned),
    ] {
        actions.on(
            action,
            move || panel.set(None),
            move || panel.set(Some(destination)),
        );
    }
    actions.on(
        "Close",
        move || panel.set(Some(Panel::Info)),
        move || panel.set(None),
    );
    let row = fixtures::conversations::stress_rows().remove(0);
    let mut list = chat::DetachedMessages::new(signal(fixtures::messages::reply_rows()));
    messages::timeline_view(chat::preview_chrome(
        row,
        signal(Some("Ava pinned \"Release at 5 pm\"".into())),
        move |top, bottom| {
            list.insets = (top, bottom);
            chat::detached_message_list(
                fixtures::USER_ID,
                list,
                true,
                fixtures::messages::mention_names(),
            )
        },
        super::composer::editor(false, actions),
    ))
    .child(inspector::overlay(panel, move |destination| {
        let close = move || panel.set(None);
        match destination {
            Panel::Info => {
                let mut row = fixtures::conversations::stress_rows().remove(0);
                row.category = "GROUP".into();
                row.name = "设计评审".into();
                crate::conversation::preview_page(
                    "info",
                    fixtures::USER_ID.into(),
                    row,
                    fixtures::participants(),
                    fixtures::users(),
                    signal(false),
                    fixtures::messages::photo_rows(),
                    close,
                )
                .into_element()
            }
            Panel::Search => inspector::preview_search(
                fixtures::messages::text_rows(),
                fixtures::participants(),
                signal("Reset"),
                close,
            )
            .into_element(),
            Panel::Pinned => inspector::preview_pinned(
                fixtures::messages::text_rows(),
                fixtures::USER_ID.into(),
                signal(false),
                close,
            )
            .into_element(),
        }
    }))
}
