use super::Handle;
use crate::{
    i18n::{self, Key},
    navigation::ChatNavigation,
    platform::notifications::{self as native, Permission},
    presentation, runtime,
};
use futures::StreamExt as _;
use mixin_desktop_api::{AccountClient, ConversationListData};
use nsh_core::{platform::Lifecycle, prelude::*};
use std::{collections::VecDeque, sync::Arc};
const MAX_SESSION_NOTIFICATIONS: usize = 256;

pub(crate) fn attach(
    account: Arc<AccountClient>,
    selected: Signal<Option<ConversationListData>>,
    navigation: ChatNavigation,
) {
    let handle = use_context::<Handle>().expect("App provides native notifications");
    if handle.native.with(|native| native.is_none()) {
        return;
    }
    let security = use_context::<crate::security::Handle>().expect("Home provides screen privacy");
    let session = Owner::current();
    let account_id = account.account_id();
    let delivered = stored(VecDeque::<(String, String)>::new());
    let client = account.clone();
    let current_account = account_id.clone();
    effect(move || {
        let clicks = handle.clicks.get();
        if clicks.is_empty() {
            return;
        }
        let latest = clicks
            .into_iter()
            .rev()
            .find(|click| click.account_id == current_account);
        handle.clicks.set(VecDeque::new());
        if let Some(click) = latest {
            use_context::<crate::shell::Handle>()
                .expect("App provides native shell")
                .show();
            navigation.open_conversation(
                client.clone(),
                click.conversation_id,
                Some(click.message_id),
                handle.failed,
            );
        }
    });
    effect(move || {
        if lifecycle() != Lifecycle::Active || security.blocked() {
            return;
        }
        let selected_id = selected.with(|row| row.as_ref().map(|row| row.conversation_id.clone()));
        let Some(selected_id) = selected_id else {
            return;
        };
        let ids = delivered.with(|rows| {
            let ids = rows
                .iter()
                .filter(|(_, id)| *id == selected_id)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            rows.retain(|(_, id)| *id != selected_id);
            ids
        });
        handle.dismiss(&ids);
    });
    let current_account = account_id.clone();
    let i18n = i18n::current();
    {
        let account = account.clone();
        let account_id = current_account.clone();
        spawn(async move {
            let mut events = Box::pin(runtime::observe(account.notification_events()));
            while let Some(event) = events.next().await {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => {
                        log::warn!("Account notification subscription failed: {error}");
                        handle.failed.set(true);
                        break;
                    }
                };
                if let Some(recalled) = event.dismiss_message_id {
                    let id = notification_id(&account_id, &recalled);
                    handle.dismiss(std::slice::from_ref(&id));
                    delivered.with(|rows| rows.retain(|(existing, _)| *existing != id));
                    continue;
                }
                let age = chrono::Utc::now()
                    .timestamp_micros()
                    .saturating_sub(event.created_at_micros);
                if age > 120_000_000 || event.created_at_micros <= 0 {
                    continue;
                }
                let active = lifecycle() == Lifecycle::Active && !security.blocked();
                if active
                    && selected.with(|selected| {
                        selected
                            .as_ref()
                            .is_some_and(|row| row.conversation_id == event.conversation_id)
                    })
                {
                    continue;
                }
                if !handle
                    .permission
                    .get_untracked()
                    .is_some_and(Permission::permits_delivery)
                {
                    continue;
                }
                let id = notification_id(&account_id, &event.message_id);
                let preview = if handle.preview.get_untracked() {
                    let content = event.content.chars().take(512).collect::<String>();
                    let body = if matches!(event.status.as_str(), "UNKNOWN" | "FAILED") {
                        i18n.format("Waiting for {sender} to get online and establish an encrypted session.",
                            "正在等待{sender}上线以建立加密会话。", &[("sender", &event.sender_name)])
                    } else if event.category == "SYSTEM_CONVERSATION" {
                        let sender = presentation::system_message::actor(
                            i18n,
                            &event.sender_id,
                            &event.sender_name,
                            &account_id,
                        );
                        let participant = presentation::system_message::actor(
                            i18n,
                            event.participant_id.as_deref().unwrap_or_default(),
                            event.participant_name.as_deref().unwrap_or_default(),
                            &account_id,
                        );
                        presentation::system_message::text(
                            i18n,
                            &sender,
                            &participant,
                            event.action.as_deref(),
                            &event.content,
                        )
                    } else if event.category == "MESSAGE_PIN" {
                        let pin = serde_json::from_str::<serde_json::Value>(&event.content).ok();
                        let message = pin
                            .as_ref()
                            .map(|pin| {
                                presentation::preview(
                                    i18n,
                                    pin["category"].as_str().unwrap_or_default(),
                                    pin["content"].as_str().unwrap_or_default(),
                                )
                            })
                            .filter(|value| !value.is_empty())
                            .unwrap_or_else(|| i18n.tr("a message", "一条消息").into());
                        i18n.format(
                            "{sender} pinned {message}",
                            "{sender}置顶了{message}",
                            &[("sender", &event.sender_name), ("message", &message)],
                        )
                    } else {
                        presentation::preview(i18n, &event.category, &content)
                    };
                    if event.conversation_category == "GROUP"
                        && !event.sender_name.is_empty()
                        && !matches!(
                            event.category.as_str(),
                            "MESSAGE_PIN" | "SYSTEM_CONVERSATION"
                        )
                    {
                        format!("{}: {body}", event.sender_name)
                    } else {
                        body
                    }
                } else {
                    i18n.text(Key::Message).to_owned()
                };
                let payload = native::Notification {
                    id: id.clone(),
                    account_id: account_id.clone(),
                    conversation_id: event.conversation_id.clone(),
                    message_id: event.message_id,
                    title: event.conversation_name,
                    body: preview,
                };
                let completion = handle
                    .native
                    .with(|native| native.as_ref().map(|native| native.deliver(payload)));
                let Some(completion) = completion else {
                    continue;
                };
                let conversation_id = event.conversation_id.clone();
                let evicted = delivered.with(|rows| {
                    let evicted = if rows.len() == MAX_SESSION_NOTIFICATIONS {
                        rows.pop_front().map(|(id, _)| id)
                    } else {
                        None
                    };
                    rows.push_back((id.clone(), event.conversation_id));
                    evicted
                });
                if let Some(evicted) = evicted {
                    handle.dismiss(&[evicted]);
                }
                // Await on the root owner so logout can dismiss a request even
                // when the OS accepts it after the account view is disposed.
                handle.owner.run(|| {
                    spawn(async move {
                        let result = completion.await;
                        if !session.is_live() {
                            handle.dismiss(&[id]);
                            return;
                        }
                        if lifecycle() == Lifecycle::Active
                            && !security.blocked()
                            && selected.with(|row| {
                                row.as_ref()
                                    .is_some_and(|row| row.conversation_id == conversation_id)
                            })
                        {
                            handle.dismiss(&[id]);
                        }
                        if let Err(error) = result {
                            log::warn!("Notification request failed: {error}");
                            handle.failed.set(true);
                        }
                    })
                });
            }
        });
    }
    cleanup(move || {
        let ids = delivered.with(|rows| rows.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>());
        handle.dismiss(&ids);
        handle.clear_account(account_id);
    });
}

fn notification_id(account_id: &str, message_id: &str) -> String {
    format!("{account_id}:{message_id}")
}
