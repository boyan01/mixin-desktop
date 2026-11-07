use mixin_desktop_api::AccountClient;
use nsh_core::prelude::*;
use std::sync::Arc;

use super::parse::Recipient;

pub(super) fn screen(
    account: Arc<AccountClient>,
    cancel: impl Fn() + Copy + 'static,
    choose: impl Fn(Recipient) + Copy + 'static,
) -> impl IntoElement {
    crate::recipients::screen(
        account,
        || false,
        cancel,
        move |value| {
            choose(match value {
                crate::recipients::Recipient::Chat(row) => {
                    Recipient::Conversation(row.conversation_id)
                }
                crate::recipients::Recipient::User(user) => Recipient::User(user.user_id),
            });
        },
    )
}
