//! The production floating inspector over chat chrome and fixed message rows.
use nsh_core::prelude::*;

use super::{Actions, fixtures, messages};
use crate::{chat, inspector, navigation::Panel};

pub(super) const ACTIONS: &[&str] = &[
    "Open info",
    "Open search",
    "Open pinned",
    "Close",
    "Fold pin",
    "Expand pin",
    "Remove pin",
    "Replace pin",
    "Long pin",
];

pub(super) fn scene(actions: Actions) -> View {
    messages::provide_services();
    let navigation = use_context::<crate::navigation::ChatNavigation>().unwrap();
    let panel = navigation.panel;
    on_key_with(
        "escape",
        KeyOptions {
            when: nsh_core::keymap::When::NotEditing,
            ..KeyOptions::default()
        },
        move |_| navigation.back_panel(),
    );
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
    let pin = signal(Some("Ava pinned \"Release at 5 pm\"".into()));
    let shown = signal(true);
    provide_context(chat::pinned::PreviewVisibility(shown));
    actions.on(
        "Fold pin",
        move || shown.set(true),
        move || shown.set(false),
    );
    actions.on(
        "Expand pin",
        move || shown.set(false),
        move || shown.set(true),
    );
    actions.on(
        "Remove pin",
        move || {
            shown.set(false);
            pin.set(Some("Ava pinned \"Release at 5 pm\"".into()));
        },
        move || pin.set(None),
    );
    actions.on(
        "Replace pin",
        move || {
            shown.set(false);
            pin.set(Some("Ava pinned \"Release at 5 pm\"".into()));
        },
        move || pin.set(Some("Carlos pinned \"Updated release plan\"".into())),
    );
    actions.on(
        "Long pin",
        move || shown.set(true),
        move || pin.set(Some("Carlos pinned the updated release plan: review the desktop header, check the narrow window layout, and confirm the final release schedule with everyone".into())),
    );
    let mut row = fixtures::conversations::stress_rows().remove(0);
    row.name = "Desktop design and release planning".into();
    let mut list = chat::DetachedMessages::new(signal(fixtures::messages::reply_rows()));
    messages::timeline_view(chat::preview_chrome(
        row,
        pin,
        move |top, bottom| {
            list.insets = (top, bottom);
            chat::detached_message_list(
                fixtures::USER_ID,
                list,
                true,
                fixtures::messages::mention_names(),
            )
        },
        super::composer::editor_with_escape(false, actions, move || navigation.back_panel()),
    ))
    .child(inspector::overlay(panel, move |destination| {
        let close = move || navigation.back_panel();
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
