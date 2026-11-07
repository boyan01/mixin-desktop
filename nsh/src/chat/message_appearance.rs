//! Shared sender, delivery and surface presentation for timeline rows.
use mixin_desktop_api::MessageListView;
use nsh_core::{element::AdaptiveColor, prelude::*};

use super::message_actions::Actions;
use crate::{
    appearance::{Appearance, identity_badge},
    ui::{self, Typography},
};

pub(super) fn bubble_surface(mine: bool) -> AdaptiveColor {
    if mine {
        ui::bubble_out()
    } else {
        ui::bubble_in()
    }
}

/// The disc under a control floating over the timeline. A state fill replaces a
/// button's own, so the surface sits on a parent and hover stacks on it.
pub(super) fn floating_disc() -> View {
    view().rounded(ui::radius_full()).bg(ui::bubble_in)
}

pub(super) fn sender(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
) -> impl IntoElement {
    let appearance = use_context::<Appearance>().expect("Home provides chat appearance");
    ui::command()
        .justify_start()
        .min_w_0()
        .min_h_0()
        .px_0()
        .py_0()
        .gap_1()
        .on_press(move || {
            if let Some(row) = message() {
                actions.show_user(row.sender_id);
            }
        })
        .child(
            text(move || message().map(|row| row.sender_name).unwrap_or_default())
                .text_size(move || appearance.secondary_size())
                .text_color(ui::accent)
                .number_of_lines(1),
        )
        .child(when(
            move || appearance.show_identity(),
            move || {
                text(move || {
                    message()
                        .and_then(|row| row.sender_identity_number)
                        .filter(|id| !id.is_empty() && id != "0")
                        .map(|id| format!("@{id}"))
                        .unwrap_or_default()
                })
                .text_role(ui::TextRole::Micro)
                .text_color(ui::fg_secondary)
            },
        ))
        .child(dynamic(move || {
            let row = message();
            untracked(|| match row {
                Some(row) => identity_badge(
                    false,
                    row.sender_is_verified,
                    row.sender_is_bot,
                    row.sender_membership.as_deref(),
                ),
                None => view().hidden().into_element(),
            })
        }))
        .child(when(
            move || message().is_some_and(|row| row.sender_is_scam),
            move || {
                text(move || actions.i18n.tr("SCAM", "诈骗风险"))
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::danger)
            },
        ))
}

pub(super) fn delivery_status(status: Memo<String>) -> impl IntoElement {
    dynamic(move || {
        let status = status.get();
        untracked(|| crate::appearance::delivery_icon(&status))
    })
}
