mod datetime;
pub(crate) mod message_window;
pub(crate) mod system_message;

use crate::i18n::{I18n, Key};
use chrono::{DateTime, Local};
use mixin_desktop_api::ConversationListData;
use nsh_base::text_view::Parser;

pub(crate) fn conversation_preview(
    i18n: I18n,
    row: &ConversationListData,
    current_user_id: &str,
) -> String {
    let Some(category) = row.last_message_category.as_deref() else {
        return String::new();
    };
    let sender_id = row.last_message_sender_id.as_deref().unwrap_or_default();
    let sender = system_message::actor(
        i18n,
        sender_id,
        row.last_message_sender_name.as_deref().unwrap_or_default(),
        current_user_id,
    );
    if category == "SYSTEM_CONVERSATION" {
        let participant = system_message::actor(
            i18n,
            row.last_message_participant_id
                .as_deref()
                .unwrap_or_default(),
            row.last_message_participant_name
                .as_deref()
                .unwrap_or_default(),
            current_user_id,
        );
        return system_message::text(
            i18n,
            &sender,
            &participant,
            row.last_message_action.as_deref(),
            &row.last_message,
        );
    }
    if category == "MESSAGE_PIN" {
        let pin = serde_json::from_str::<serde_json::Value>(&row.last_message).ok();
        let message = pin
            .as_ref()
            .map(|pin| {
                preview(
                    i18n,
                    pin["category"].as_str().unwrap_or_default(),
                    pin["content"].as_str().unwrap_or_default(),
                )
            })
            .filter(|message| !message.is_empty())
            .unwrap_or_else(|| i18n.tr("a message", "一条消息").into());
        return i18n.format(
            "{sender} pinned {message}",
            "{sender}置顶了{message}",
            &[("sender", &sender), ("message", &message)],
        );
    }
    let body = if row.last_message_status.as_deref() == Some("FAILED") {
        i18n.tr("Waiting for this message.", "正在等待此消息。")
            .into()
    } else if row.last_message_status.as_deref() == Some("UNKNOWN") {
        i18n.tr("This message type is not supported.", "暂不支持此类消息。")
            .into()
    } else if category == "MESSAGE_RECALL" {
        if !sender_id.is_empty() && sender_id == current_user_id {
            i18n.tr("You deleted this message", "你删除了这条消息")
                .into()
        } else {
            i18n.tr("This message was deleted", "这条消息已被删除")
                .into()
        }
    } else if category.ends_with("_POST") {
        let source = row
            .last_message
            .lines()
            .take(10)
            .flat_map(|line| line.chars().chain(std::iter::once('\n')))
            .take(1024)
            .collect::<String>();
        Parser::new()
            .markdown(&source)
            .plain_text()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        preview(i18n, category, row.last_message.trim())
    };
    if !body.is_empty()
        && (row.category == "GROUP" || (!sender_id.is_empty() && sender_id != row.owner_id))
    {
        format!("{sender}: {body}")
    } else {
        body
    }
}

pub(crate) fn preview(i18n: I18n, category: &str, content: &str) -> String {
    if category.ends_with("_TEXT") {
        return content.to_owned();
    }
    let key = if category.ends_with("_IMAGE") {
        Key::Photo
    } else if category.ends_with("_VIDEO") {
        Key::Video
    } else if category.ends_with("_AUDIO") {
        Key::Audio
    } else if category.ends_with("_DATA") {
        Key::File
    } else if category.ends_with("_STICKER") {
        Key::Sticker
    } else if category.ends_with("_CONTACT") {
        Key::Contact
    } else if category.ends_with("_LOCATION") {
        Key::Location
    } else if category.ends_with("_POST") {
        Key::Post
    } else if category.ends_with("_TRANSCRIPT") {
        Key::Transcript
    } else if category.starts_with("SYSTEM_ACCOUNT_SNAPSHOT") {
        Key::Transfer
    } else {
        Key::Message
    };
    i18n.text(key).to_owned()
}

pub(crate) fn time(i18n: I18n, millis: i64) -> String {
    let Some(date) = DateTime::from_timestamp_millis(millis).filter(|_| millis > 0) else {
        return String::new();
    };
    let date = date.with_timezone(&Local);
    let today = Local::now().date_naive();
    if date.date_naive() == today {
        date.format("%H:%M").to_string()
    } else {
        datetime::short(date.date_naive(), today, i18n.locale_tag())
    }
}

pub(crate) fn date(i18n: I18n, millis: i64) -> String {
    DateTime::from_timestamp_millis(millis)
        .filter(|_| millis > 0)
        .map(|date| datetime::full(date.with_timezone(&Local).date_naive(), i18n.locale_tag()))
        .unwrap_or_default()
}

/// A message's time of day: its row already sits under its day.
pub(crate) fn clock(millis: i64) -> String {
    DateTime::from_timestamp_millis(millis)
        .filter(|_| millis > 0)
        .map(|date| date.with_timezone(&Local).format("%H:%M").to_string())
        .unwrap_or_default()
}

pub(crate) fn day(i18n: I18n, micros: i64) -> String {
    let Some(date) = DateTime::from_timestamp_micros(micros) else {
        return String::new();
    };
    let date = date.with_timezone(&Local).date_naive();
    let today = Local::now().date_naive();
    if date == today {
        i18n.text(Key::Today).into()
    } else if today.signed_duration_since(date).num_days() == 1 {
        i18n.text(Key::Yesterday).into()
    } else {
        datetime::day(date, today, i18n.locale_tag())
    }
}
