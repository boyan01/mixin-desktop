use super::message_actions::Actions;
use crate::{
    i18n::I18n,
    presentation, runtime,
    ui::{self, Typography},
};
use mixin_desktop_api::MessageListView;
use nsh_core::prelude::*;

pub(super) fn centered(category: &str) -> bool {
    matches!(
        category,
        "SYSTEM_CONVERSATION" | "MESSAGE_PIN" | "SECRET" | "STRANGER"
    )
}

pub(super) fn supported(category: &str) -> bool {
    if matches!(
        category,
        "MESSAGE_RECALL"
            | "SYSTEM_ACCOUNT_SNAPSHOT"
            | "SYSTEM_SAFE_SNAPSHOT"
            | "SYSTEM_SAFE_INSCRIPTION"
            | "APP_CARD"
            | "APP_BUTTON_GROUP"
    ) || centered(category)
    {
        return true;
    }
    category.split_once('_').is_some_and(|(prefix, kind)| {
        matches!(prefix, "PLAIN" | "SIGNAL" | "ENCRYPTED")
            && matches!(
                kind,
                "TEXT"
                    | "POST"
                    | "STICKER"
                    | "IMAGE"
                    | "VIDEO"
                    | "AUDIO"
                    | "DATA"
                    | "CONTACT"
                    | "TRANSCRIPT"
                    | "LIVE"
                    | "LOCATION"
            )
    })
}

pub(super) fn unavailable(row: &MessageListView, actions: Actions) -> Option<AnyElement> {
    let i18n = actions.i18n;
    let unsupported = row.status == "UNKNOWN" || !supported(&row.category);
    if !unsupported && row.status != "FAILED" {
        return None;
    }
    let description = if unsupported {
        i18n.tr("This message type is not supported.", "暂不支持此类消息。")
            .to_owned()
    } else {
        let sender = if row.sender_relationship == "ME" {
            i18n.tr("your linked device", "你的关联设备")
        } else if row.sender_name.is_empty() {
            i18n.tr("the sender", "发送者")
        } else {
            &row.sender_name
        };
        i18n.format(
            "Waiting for {sender} to get online and establish an encrypted session.",
            "等待{sender}上线后建立加密会话。",
            &[("sender", sender)],
        )
    };
    Some(
        view()
            .gap_1()
            .child(text(description))
            .child(help(actions, false))
            .into_element(),
    )
}

fn help(actions: Actions, encryption: bool) -> impl IntoElement {
    ui::link(
        move || actions.i18n.tr("Learn more", "了解更多"),
        move || {
            let url = if encryption {
                "https://mixin.one/pages/1000007"
            } else if actions.i18n.chinese() {
                "https://support.mixin.one/zh/article/5ye6546w4occ6lz5liq57g75z6l55qe5rai5ogv5lin5psv5oyb4ocd5oco5lmi5yqe77yf-h92cxa/"
            } else {
                "https://support.mixin.one/en/article/how-to-do-when-you-receive-a-message-like-this-this-type-of-message-is-not-supported-please-upgrade-mixin-17j1t3p"
            };
            actions.task(async move {
                actions.failed.set(!open_url(url).await);
            });
        },
    )
}

pub(super) fn notice(
    row: &MessageListView,
    actions: Actions,
    target: Option<Signal<Option<String>>>,
) -> AnyElement {
    let i18n = actions.i18n;
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let mut content = view()
        .gap_1()
        .px_3()
        .py_2()
        .rounded(ui::radius_md())
        .max_w(ui::bubble_width())
        .bg(ui::day_background)
        .text_size(move || appearance.secondary_size());
    match row.category.as_str() {
        "SYSTEM_CONVERSATION" => content = content.child(text(system_text(row, actions))),
        "MESSAGE_PIN" => {
            let pin = serde_json::from_str::<serde_json::Value>(&row.content).ok();
            let preview = pin
                .as_ref()
                .map(|pin| {
                    presentation::preview(
                        i18n,
                        pin.get("category")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default(),
                        pin.get("content")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default(),
                    )
                })
                .filter(|preview| !preview.is_empty())
                .unwrap_or_else(|| i18n.tr("a message", "一条消息").into());
            let sender = actor(i18n, &row.sender_id, &row.sender_name, actions);
            let description = i18n.format(
                "{sender} pinned {message}",
                "{sender}置顶了{message}",
                &[
                    ("sender", &sender),
                    ("message", preview.lines().next().unwrap_or_default()),
                ],
            );
            let id = pin.and_then(|pin| {
                pin.get("message_id")
                    .and_then(|id| id.as_str())
                    .map(str::to_owned)
            });
            if let Some((target, id)) = target.zip(id).filter(|(_, id)| !id.is_empty()) {
                content = content.child(
                    ui::command()
                        .px_0()
                        .py_0()
                        .on_press(move || target.set(Some(id.clone())))
                        .child(ui::icon(ui::Icon::Pin))
                        .child(text(description).number_of_lines(1)),
                );
            } else {
                content = content.child(text(description).number_of_lines(1));
            }
        }
        "SECRET" => return encryption_notice(actions).into_element(),
        "STRANGER" => content = content.child(stranger(row.clone(), actions)),
        _ => {}
    }
    view().items_center().child(content).into_element()
}

pub(super) fn encryption_notice(actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    view().items_center().px_2().py_2().child(
        view()
            .gap_1()
            .px_3()
            .py_2()
            .rounded(ui::radius_md())
            .max_w(ui::bubble_width())
            .bg(ui::day_background)
            .child(
                text(move || {
                    i18n.tr(
                        "Messages in this conversation are end-to-end encrypted.",
                        "此对话中的消息使用端对端加密。",
                    )
                })
                .text_size(move || appearance.secondary_size()),
            )
            .child(help(actions, true)),
    )
}

fn actor(i18n: I18n, id: &str, name: &str, actions: Actions) -> String {
    presentation::system_message::actor(
        i18n,
        id,
        name,
        &actions.account.with(|account| account.account_id()),
    )
}

fn system_text(row: &MessageListView, actions: Actions) -> String {
    let i18n = actions.i18n;
    let sender = actor(i18n, &row.sender_id, &row.sender_name, actions);
    let participant = actor(
        i18n,
        row.participant_id.as_deref().unwrap_or_default(),
        row.participant_full_name.as_deref().unwrap_or_default(),
        actions,
    );
    presentation::system_message::text(
        i18n,
        &sender,
        &participant,
        row.action.as_deref(),
        &row.content,
    )
}

fn stranger(row: MessageListView, actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let bot = row.sender_app_id.is_some();
    let router =
        use_context::<crate::links::LinkNavigation>().expect("Home provides link navigation");
    let contact = stored((
        row.sender_id,
        row.sender_name,
        row.sender_app_id,
        row.conversation_id,
    ));
    view()
        .gap_2()
        .child(text(move || {
            if bot {
                i18n.tr(
                    "Send a message to start using this bot.",
                    "发送消息以开始使用此机器人。",
                )
            } else {
                i18n.tr(
                    "This person is not in your contacts.",
                    "对方不在你的联系人中。",
                )
            }
        }))
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .child(
                    ui::action(
                        move || {
                            if bot {
                                i18n.tr("Open home page", "打开主页")
                            } else {
                                i18n.tr("Block", "屏蔽")
                            }
                        },
                        move || {
                            let client = actions.account.with(|account| account.clone());
                            let (user_id, _, app_id, conversation) =
                                contact.with(|contact| contact.clone());
                            actions.busy.set(true);
                            actions.failed.set(false);
                            actions.task(async move {
                                if let Some(app_id) = app_id {
                                    match runtime::run(async move {
                                        client.user().bot_home_uri(app_id).await
                                    })
                                    .await
                                    {
                                        Ok(Some(uri)) => router.open(uri, Some(conversation)),
                                        _ => actions.failed.set(true),
                                    }
                                } else {
                                    actions.failed.set(
                                        runtime::commit(async move {
                                            client.user().block_user(user_id).await
                                        })
                                        .await
                                        .is_err(),
                                    );
                                }
                                actions.busy.set(false);
                            });
                        },
                    )
                    .disabled(move || actions.busy.get()),
                )
                .child(
                    ui::action(
                        move || {
                            if bot {
                                i18n.tr("Say Hi", "打招呼")
                            } else {
                                i18n.tr("Add contact", "添加联系人")
                            }
                        },
                        move || {
                            if bot && !actions.can_send.get_untracked() {
                                return;
                            }
                            let client = actions.account.with(|account| account.clone());
                            let (user_id, name, _, conversation) =
                                contact.with(|contact| contact.clone());
                            actions.busy.set(true);
                            actions.failed.set(false);
                            actions.task(async move {
                                let result = runtime::commit(async move {
                                    if bot {
                                        client
                                            .message()
                                            .send_text(conversation, "Hi".into(), None, false)
                                            .await
                                            .map(|_| ())
                                    } else {
                                        client.user().add_contact(user_id, name).await
                                    }
                                })
                                .await;
                                actions.failed.set(result.is_err());
                                actions.busy.set(false);
                            });
                        },
                    )
                    .disabled(move || actions.busy.get() || (bot && !actions.can_send.get())),
                ),
        )
}

pub(super) fn inscription(row: &MessageListView, actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let message = row.clone();
    let mut card = ui::command()
        .flex_col()
        .items_start()
        .gap_2()
        .disabled(row.snapshot_id.is_none())
        .on_press(move || actions.open_preview(message.clone()))
        .child(
            text(
                row.inscription_name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| i18n.tr("Inscription", "铭文").into()),
            )
            .text_emphasis(),
        );
    if let Some(url) = row.inscription_content_url.as_ref().filter(|_| {
        row.inscription_content_type
            .as_deref()
            .is_some_and(|kind| kind.starts_with("image/"))
    }) {
        card = card.child(
            image(url.clone())
                .size_36()
                .object_fit(ResizeMode::Contain)
                .label(move || i18n.tr("Inscription", "铭文")),
        );
    }
    if let Some(sequence) = row.inscription_sequence {
        card = card.child(text(format!("#{sequence}")).text_role(ui::TextRole::Body));
    }
    if let Some(hash) = row
        .inscription_hash
        .as_ref()
        .filter(|hash| !hash.is_empty())
    {
        card = card.child(
            text(hash.clone())
                .number_of_lines(1)
                .text_role(ui::TextRole::Caption)
                .text_color(ui::fg_secondary),
        );
    } else {
        card = card.child(
            text(move || {
                i18n.tr(
                    "Inscription details are not available yet.",
                    "铭文详情暂不可用。",
                )
            })
            .text_role(ui::TextRole::Body),
        );
    }
    card
}
