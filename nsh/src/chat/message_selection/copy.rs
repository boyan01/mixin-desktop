//! Copy one selection from authoritative rows without retaining a second message window.
use std::sync::Arc;

use chrono::{DateTime, Local};
use mixin_desktop_api::{AccountClient, ClientError, ClientResult, MessageListView};
use nsh_base::text_view::Parser;
use nsh_core::prelude::*;

use crate::{
    chat::{message_actions::Actions, message_system},
    i18n::I18n,
    presentation, runtime,
};

pub(super) fn selected(actions: Actions, conversation: String) {
    if actions.busy.get_untracked() {
        return;
    }
    let ids = actions.selected.get_untracked();
    if ids.is_empty() {
        return;
    }
    let account = actions.account.with(|account| account.clone());
    actions.busy.set(true);
    actions.failed.set(false);
    actions.task(async move {
        match selection_text(account, &conversation, &ids, actions.i18n).await {
            Ok(text) => {
                // NSH exposes clipboard text as a native command without an OS
                // completion reply. Dispatch only after the query has succeeded;
                // never read the clipboard back to infer success.
                set_clipboard(text);
                actions.selected.update(|current| {
                    if *current == ids {
                        current.clear();
                    }
                });
            }
            Err(_) => actions.failed.set(true),
        }
        actions.busy.set(false);
    });
}

async fn selection_text(
    account: Arc<AccountClient>,
    conversation: &str,
    ids: &[String],
    i18n: I18n,
) -> ClientResult<String> {
    let mut lines = Vec::new();
    // Only one bounded batch of screen DTOs is retained. The final text and its
    // ordering keys are the temporary clipboard payload, not a durable UI cache.
    for batch in ids.chunks(100) {
        let requested = batch.to_vec();
        let account = account.clone();
        let rows =
            runtime::run(async move { account.message().message_items_by_ids(requested).await })
                .await?;
        for row in rows {
            if row.conversation_id != conversation {
                continue;
            }
            let body = content(&row, i18n);
            let date = timestamp(row.created_at_micros, i18n.locale_tag());
            let text = format!("{}, ({date}):\n{body}", row.sender_name);
            lines.push((row.created_at_micros, row.message_id, text));
        }
    }
    if lines.is_empty() {
        return Err(ClientError::NotFound);
    }
    lines.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    Ok(lines
        .into_iter()
        .map(|(_, _, text)| text)
        .collect::<Vec<_>>()
        .join("\n\n"))
}

fn timestamp(micros: i64, locale: &str) -> String {
    let Some(date) = DateTime::from_timestamp_micros(micros) else {
        return String::new();
    };
    // intl 0.20.3's yMd().add_Hms() skeletons from the Flutter reference.
    let pattern = match locale {
        "es" => "%-d/%-m/%Y %-H:%M:%S",
        "id" => "%-d/%-m/%Y %H.%M.%S",
        "ms" | "zh-HK" => "%-d/%-m/%Y %H:%M:%S",
        "ru" => "%d.%m.%Y %H:%M:%S",
        "ja" => "%Y/%-m/%-d %-H:%M:%S",
        "zh-CN" | "zh-TW" => "%Y/%-m/%-d %H:%M:%S",
        _ => "%-m/%-d/%Y %H:%M:%S",
    };
    date.with_timezone(&Local).format(pattern).to_string()
}

fn content(row: &MessageListView, i18n: I18n) -> String {
    if row.status == "UNKNOWN" || !message_system::supported(&row.category) {
        return i18n
            .tr("This message type is not supported.", "暂不支持此类消息。")
            .into();
    }
    if row.status == "FAILED" {
        return i18n
            .tr("Waiting for this message.", "正在等待此消息。")
            .into();
    }
    if row.category.ends_with("_TEXT") {
        return row.content.clone();
    }
    if row.category.ends_with("_POST") {
        let source = row
            .content
            .lines()
            .take(10)
            .flat_map(|line| line.chars().chain(std::iter::once('\n')))
            .take(1024)
            .collect::<String>();
        return Parser::new()
            .markdown(&source)
            .plain_text()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    }
    if row.category == "APP_CARD" {
        let value = serde_json::from_str::<serde_json::Value>(&row.content).ok();
        let title = value
            .as_ref()
            .and_then(|value| value.get("title"))
            .and_then(serde_json::Value::as_str);
        return format!("[{}]", title.unwrap_or_else(|| i18n.tr("Card", "卡片")));
    }
    if row.category == "APP_BUTTON_GROUP" {
        return serde_json::from_str::<Vec<serde_json::Value>>(&row.content)
            .unwrap_or_default()
            .iter()
            .filter_map(|button| button.get("label").and_then(serde_json::Value::as_str))
            .map(|label| format!("[{label}]"))
            .collect::<Vec<_>>()
            .join("");
    }
    if row.category == "MESSAGE_RECALL" {
        return format!(
            "[{}]",
            i18n.tr("This message was deleted", "这条消息已被删除")
        );
    }
    if row.category.ends_with("_LIVE") {
        return format!("[{}]", i18n.tr("Live", "直播"));
    }
    if row.category == "SYSTEM_SAFE_INSCRIPTION" {
        return format!("[{}]", i18n.tr("Collectible", "藏品"));
    }
    format!(
        "[{}]",
        presentation::preview(i18n, &row.category, &row.content)
    )
}
