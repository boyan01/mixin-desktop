use super::message_actions::Actions;
use crate::{
    i18n,
    recipients::{self, Recipient},
    runtime, ui,
};
use nsh_core::prelude::*;

pub(super) fn forwarding(actions: Actions) -> impl IntoElement {
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
            let send = move |target: Recipient| {
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
            };
            let picker = move || {
                recipients::screen(
                    actions.account.with(|account| account.clone()),
                    move || actions.busy.get() || !presence.interactive(),
                    cancel,
                    send,
                )
            };
            ui::dialog(presence, cancel, form(actions.busy, failed, cancel, picker))
        },
    )
}

fn form<E: IntoElement>(
    busy: Signal<bool>,
    failed: Signal<bool>,
    cancel: impl Fn() + Copy + 'static,
    picker: impl Fn() -> E + 'static,
) -> View {
    let i18n = i18n::current();
    ui::picker::dialog()
        .child(ui::picker::header(
            move || i18n.tr("Forward messages", "转发消息"),
            String::new,
            ui::picker::close(cancel).disabled(busy),
            view().hidden(),
        ))
        .child(picker())
        .child(when(failed, move || {
            ui::error_text(move || {
                i18n.tr(
                    "Couldn’t forward. Choose a chat to try again.",
                    "转发失败，请重新选择聊天重试。",
                )
            })
            .px(ui::space_xl())
            .pb(ui::space_lg())
        }))
}

#[cfg(debug_assertions)]
pub(crate) fn detached(
    users: Vec<mixin_desktop_api::UserProfileItem>,
    failed: Signal<bool>,
) -> impl IntoElement {
    let users = stored(users);
    form(
        signal(false),
        failed,
        || {},
        move || recipients::detached(users.get(), |_| false, |_| {}, false),
    )
}
