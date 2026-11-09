use super::message_actions::Actions;
use crate::ui;
use mixin_desktop_api::MessageListView;
use nsh_core::prelude::*;

pub(super) fn screen(message: MessageListView, actions: Actions) -> impl IntoElement {
    let Some(id) = message.snapshot_id else {
        return text(move || actions.i18n.text(crate::i18n::Key::CommandFailed)).into_element();
    };
    let reference = if matches!(
        message.category.as_str(),
        "SYSTEM_SAFE_SNAPSHOT" | "SYSTEM_SAFE_INSCRIPTION"
    ) {
        crate::snapshot::Reference::Safe(id)
    } else {
        crate::snapshot::Reference::Snapshot(id)
    };
    crate::snapshot::screen(
        actions.account.with(|account| account.clone()),
        reference,
        move || actions.close_preview(),
        when(
            move || !actions.preview_history.with(Vec::is_empty),
            move || {
                ui::round_button()
                    .size_10()
                    .accessibility_label(move || {
                        Some(actions.i18n.text(crate::i18n::Key::Back).into())
                    })
                    .on_press(move || actions.back_preview())
                    .child(ui::icon(ui::Icon::ChevronLeft))
            },
        ),
    )
    .into_element()
}
