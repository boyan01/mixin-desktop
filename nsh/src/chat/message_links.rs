use super::message_actions::Actions;
use mixin_desktop_api::MessageListView;
use nsh_core::prelude::*;
use serde_json::Value;

use crate::{bot::BotWindows, links::AppLink, runtime};

enum CardAction {
    Input(String),
    Internal(String),
    Web(String),
    Invalid,
}

impl CardAction {
    fn parse(raw: String) -> Self {
        if let Some(content) = raw.strip_prefix("input:") {
            return Self::Input(content.trim().to_owned());
        }
        let Ok(url) = url::Url::parse(&raw) else {
            return Self::Invalid;
        };
        if url.scheme() == "mixin"
            || matches!(url.scheme(), "http" | "https")
                && matches!(url.host_str(), Some("mixin.one" | "www.mixin.one"))
        {
            Self::Internal(raw)
        } else if matches!(url.scheme(), "http" | "https") {
            Self::Web(raw)
        } else {
            Self::Invalid
        }
    }
}

/// Capture the message's origin while building the card, before navigation changes.
pub(super) fn card_action(
    actions: Actions,
    row: &MessageListView,
    raw: String,
    app_card_data: Option<Value>,
) -> impl Fn() + 'static {
    let router =
        use_context::<crate::links::LinkNavigation>().expect("Home provides link navigation");
    let windows = use_context::<BotWindows>().expect("Home provides bot windows");
    let action = CardAction::parse(raw);
    let conversation = row.conversation_id.clone();
    // Single-action shared cards carry their original app identity. The bot client
    // resolves that identity against the server before creating a window.
    let app_id = app_card_data
        .as_ref()
        .and_then(|data| data.get("app_id"))
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .or_else(|| row.sender_app_id.clone().filter(|id| !id.is_empty()))
        .or_else(|| row.sender_is_bot.then(|| row.sender_id.clone()));
    // A cold user cache cannot establish that a sender is not a bot.
    let allow_generic = app_id.is_none();
    let app_id = app_id.or_else(|| Some(row.sender_id.clone()));
    move || match &action {
        CardAction::Input(content) => {
            if content.is_empty()
                || !actions.can_send.get_untracked()
                || actions.busy.get_untracked()
                || windows.ending()
            {
                return;
            }
            actions.busy.set(true);
            actions.failed.set(false);
            let client = actions.account.with(|account| account.clone());
            let conversation = conversation.clone();
            let content = content.clone();
            // Begin the committed write before its view can be disposed.
            let sent = runtime::commit(async move {
                client
                    .message()
                    .send_text(conversation, content, None, false)
                    .await
            });
            actions.task(async move {
                actions.failed.set(sent.await.is_err());
                actions.busy.set(false);
            });
        }
        CardAction::Internal(raw) => router.open(raw.clone(), Some(conversation.clone())),
        CardAction::Web(url) => {
            windows.open(AppLink {
                app_id: app_id.clone(),
                allow_generic,
                url: url.clone(),
                source_conversation: Some(conversation.clone()),
                app_card_data: app_card_data.clone(),
            });
        }
        CardAction::Invalid => actions.failed.set(true),
    }
}

pub(super) fn open(_actions: Actions, raw: String, source_conversation: String) {
    let router =
        use_context::<crate::links::LinkNavigation>().expect("Home provides link navigation");
    router.open(raw, Some(source_conversation));
}
