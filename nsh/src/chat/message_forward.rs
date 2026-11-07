use super::message_actions::Actions;
use crate::{
    i18n::{self, Key},
    recipients::{self, Recipient},
    runtime, ui,
};
use nsh_core::prelude::*;

pub(super) fn forwarding(actions: Actions) -> impl IntoElement {
    let i18n = i18n::current();
    let presence =
        crate::motion::Presence::modal(move || actions.forwarding.with(|ids| !ids.is_empty()));
    when(
        move || presence.mounted(),
        move || {
            let failed = signal(false);
            let cancel = move || {
                if presence.interactive() && !actions.busy.get_untracked() {
                    actions.forwarding.set(Vec::new());
                }
            };
            ui::modal(
                presence,
                cancel,
                ui::modal_card()
                    .w_96()
                    .gap_3()
                    .child(ui::modal_title(move || {
                        i18n.tr("Forward messages", "转发消息")
                    }))
                    .child(recipients::screen(
                        actions.account.with(|account| account.clone()),
                        move || actions.busy.get() || !presence.interactive(),
                        cancel,
                        move |target| {
                            if !presence.interactive() || actions.busy.get_untracked() {
                                return;
                            }
                            let ids = actions.forwarding.get_untracked();
                            if ids.is_empty() {
                                return;
                            }
                            let client = actions.account.with(|account| account.clone());
                            let combine = actions.combine.get_untracked();
                            actions.busy.set(true);
                            failed.set(false);
                            // Submit the selected operation before its UI owner can be disposed.
                            let task = runtime::commit(async move {
                                let conversation_id = match target {
                                    Recipient::Chat(row) => row.conversation_id,
                                    Recipient::User(user) => {
                                        client
                                            .conversation()
                                            .open_user_conversation(user.user_id)
                                            .await?
                                    }
                                };
                                if combine {
                                    client
                                        .message()
                                        .combine_forward_messages(conversation_id, ids)
                                        .await
                                        .map(|_| ())
                                } else {
                                    client
                                        .message()
                                        .forward_messages(conversation_id, ids)
                                        .await
                                        .map(|_| ())
                                }
                            });
                            spawn(async move {
                                match task.await {
                                    Ok(_) => {
                                        actions.forwarding.set(Vec::new());
                                        actions.selected.set(Vec::new());
                                    }
                                    Err(_) => failed.set(true),
                                }
                                actions.busy.set(false);
                            });
                        },
                    ))
                    .child(when(failed, move || {
                        ui::error_text(move || i18n.text(Key::CommandFailed))
                    }))
                    .child(view().flex_none().flex_row().justify_end().gap_2().child(
                        ui::action(move || i18n.text(Key::Cancel), cancel).disabled(actions.busy),
                    )),
            )
        },
    )
}
