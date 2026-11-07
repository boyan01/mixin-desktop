use super::{
    message_actions::{Actions, label},
    message_detail, message_system, message_text,
};
use crate::{
    i18n::{self, Key},
    presentation, runtime, ui,
};
use mixin_desktop_api::MessageListView;
use nsh_base::{text_view, theme};
use nsh_core::prelude::*;

pub(super) fn is_attachment(row: &MessageListView) -> bool {
    ["_IMAGE", "_VIDEO", "_AUDIO", "_DATA"]
        .iter()
        .any(|suffix| row.category.ends_with(suffix))
}
pub(super) fn available(row: &MessageListView) -> bool {
    matches!(row.media_status.as_str(), "DONE" | "READ")
        && row.media_url.as_ref().is_some_and(|p| !p.is_empty())
}

pub(super) fn is_markdown(row: &MessageListView) -> bool {
    row.category.ends_with("_DATA")
        && row
            .media_name
            .as_deref()
            .and_then(|name| std::path::Path::new(name).extension())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

pub(super) fn body(
    row: MessageListView,
    actions: Actions,
    target: Option<Signal<Option<String>>>,
    transcript: Option<String>,
) -> AnyElement {
    let i18n = i18n::current();
    if let Some(unavailable) = message_system::unavailable(&row, actions) {
        return unavailable;
    }
    if message_system::centered(&row.category) {
        return message_system::notice(&row, actions, target);
    }
    let caption_root = transcript.clone();
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let mut content = view()
        .min_w_0()
        .gap_2()
        .text_size(move || appearance.primary_size());
    if row
        .quote_message_id
        .as_ref()
        .is_some_and(|id| !id.is_empty())
    {
        content = content.child(super::message_quote::quoted(
            &row,
            actions,
            target,
            transcript.is_some(),
        ));
    }
    if row.category.ends_with("_TEXT") {
        content = content.child(message_text::read(
            &row,
            &row.content,
            actions,
            transcript.clone(),
        ));
    } else if row.category == "MESSAGE_RECALL" {
        let reedit = use_context::<super::reedit::Reedit>().expect("Home provides recall editing");
        let mine = actions.account.with(|account| account.account_id()) == row.sender_id;
        let id = stored(row.message_id.clone());
        content = content
            .child(
                text(move || {
                    if mine {
                        label(i18n, "You deleted this message", "你删除了这条消息")
                    } else {
                        label(i18n, "This message was deleted", "这条消息已被删除")
                    }
                })
                .text_color(move || theme().muted_foreground),
            )
            .child(when(
                move || mine && id.with(|id| reedit.available(id)),
                move || {
                    ui::action(
                        move || label(i18n, "Re-edit", "重新编辑"),
                        move || id.with(|id| reedit.request(id)),
                    )
                    .px_0()
                    .py_0()
                    .text_color(move || theme().primary)
                },
            ));
    } else if row.category.ends_with("_POST") {
        let preview = row.clone();
        content = content.child(
            ui::command()
                .p_0()
                .min_h(px(48.0))
                .min_w_0()
                .w(px(400.0))
                .max_w(relative(1.0))
                .flex_col()
                .items_stretch()
                .max_h(px(400.0))
                .overflow_hidden()
                .accessibility_label(move || Some(i18n.text(Key::Post).into()))
                .on_press(move || actions.open_preview(preview.clone()))
                .child(
                    view()
                        .w_full()
                        .min_w_0()
                        .min_h_0()
                        .max_h(px(376.0))
                        .overflow_hidden()
                        .pointer_events(PointerEvents::None)
                        .child(
                            text_view()
                                .w_full()
                                .min_w_0()
                                .max_w(relative(1.0))
                                .markdown(Some(SharedString::from(post_preview(&row.content))))
                                .selectable(false)
                                .code_block_scrollbar(false)
                                .table_scrollbar(false)
                                .text_size(move || appearance.primary_size()),
                        ),
                )
                .child(
                    text(move || i18n.text(Key::Post))
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(move || theme().muted_foreground),
                ),
        );
    } else if row.category.ends_with("_STICKER") {
        if row.sticker_id.as_ref().is_some_and(|id| !id.is_empty())
            || row
                .sticker_asset_url
                .as_ref()
                .is_some_and(|url| !url.is_empty())
        {
            content = content.child(
                ui::command()
                    .p_0()
                    .min_h_0()
                    .disabled(row.sticker_id.as_ref().is_none_or(String::is_empty))
                    .accessibility_label(move || Some(i18n.text(Key::Sticker).into()))
                    .on_press({
                        let sticker = row.sticker_id.clone().unwrap_or_default();
                        move || actions.open_sticker(sticker.clone())
                    })
                    .child(crate::sticker::asset(
                        actions.account.with(|value| value.clone()),
                        {
                            let id = row.sticker_id.clone();
                            let url = row.sticker_asset_url.clone().unwrap_or_default();
                            let kind = row.sticker_asset_type.clone().unwrap_or_default();
                            move || (id.clone(), url.clone(), kind.clone())
                        },
                        160.0,
                    )),
            );
        } else {
            content = content.child(text(move || i18n.text(Key::Sticker)));
        }
    } else if row.category.ends_with("_IMAGE") || row.category.ends_with("_VIDEO") {
        content = content.child(super::message_media::card(
            row.clone(),
            transcript.clone(),
            actions,
        ));
    } else if is_attachment(&row) {
        if available(&row) {
            let path = row.media_url.clone().unwrap_or_default();
            if row.category.ends_with("_AUDIO") {
                content =
                    content.child(super::audio::body(row.clone(), transcript.clone(), actions));
            }
            let filename = row
                .media_name
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| presentation::preview(i18n, &row.category, &row.content));
            if !row.category.ends_with("_AUDIO") {
                let markdown = is_markdown(&row);
                let preview = row.clone();
                let root = transcript.clone();
                content = content.child(
                    ui::action(
                        move || label(i18n, "Open", "打开"),
                        move || {
                            if markdown {
                                actions.open_markdown(preview.clone(), root.clone());
                                return;
                            }
                            let path = path.clone();
                            spawn(async move {
                                actions.failed.set(open_path(path).await.is_err());
                            });
                        },
                    )
                    .child(
                        text(filename)
                            .text_size(move || appearance.secondary_size())
                            .number_of_lines(2),
                    ),
                );
            }
            if let Some(size) = row.media_size.filter(|size| *size >= 0)
                && !row.category.ends_with("_AUDIO")
            {
                content = content.child(
                    text(format!("{} KB", (size as u64).div_ceil(1024)))
                        .text_size(move || appearance.tertiary_size())
                        .text_color(move || theme().muted_foreground),
                );
            }
        } else {
            let id = row.message_id.clone();
            let retry_upload = transcript.is_some()
                || (row.sender_id == actions.account.with(|account| account.account_id())
                    && matches!(row.status.as_str(), "SENDING" | "FAILED"));
            let pending = matches!(row.media_status.as_str(), "PENDING" | "PROCESSING");
            let transfers = use_context::<crate::home::AttachmentProgress>()
                .expect("Home provides attachment progress");
            let progress_key = stored((transcript.clone(), row.message_id.clone()));
            let bytes =
                move || progress_key.with(|(root, id)| transfers.bytes(root.as_deref(), id));
            let transferring = move || pending || bytes().is_some();
            content = content.child(
                text(presentation::preview(i18n, &row.category, &row.content))
                    .text_size(move || appearance.primary_size()),
            );
            content = content.child(when(transferring, move || {
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(ui::busy_indicator(
                        move || bytes().is_none_or(|(_, total)| total == 0),
                        move || i18n.tr("Transferring…", "正在传输…").into(),
                    ))
                    .child(
                        text(move || match bytes() {
                            Some((completed, total)) if total > 0 => {
                                format!(
                                    "{:.0}%",
                                    (completed as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                                )
                            }
                            _ => i18n.tr("Transferring…", "正在传输…").into(),
                        })
                        .text_size(move || appearance.tertiary_size())
                        .text_color(move || theme().muted_foreground),
                    )
            }));
            content = content.child(ui::action(
                move || {
                    if transferring() {
                        i18n.tr("Cancel transfer", "取消传输")
                    } else if retry_upload {
                        i18n.tr("Retry", "重试")
                    } else {
                        i18n.tr("Download", "下载")
                    }
                },
                move || {
                    let client = actions.account.with(|a| a.clone());
                    let id = id.clone();
                    let transcript = transcript.clone();
                    let transferring = transferring();
                    spawn(async move {
                        actions.failed.set(
                            runtime::commit(async move {
                                if transferring {
                                    if let Some(transcript) = transcript {
                                        client
                                            .attachment()
                                            .cancel_transcript_attachment(transcript, id)
                                            .await
                                    } else {
                                        client.attachment().cancel_attachment(id).await
                                    }
                                } else {
                                    client.attachment().resume_attachment(id, transcript).await
                                }
                            })
                            .await
                            .is_err(),
                        );
                    });
                },
            ));
        }
    } else if row.category.ends_with("_CONTACT") {
        let user_id = row.shared_user_id.clone();
        content = content.child(
            ui::command()
                .flex_col()
                .items_start()
                .on_press(move || {
                    if let Some(id) = user_id.clone() {
                        actions.show_user(id);
                    }
                })
                .child(
                    text(row.shared_user_full_name.clone().unwrap_or_default())
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    text(
                        row.shared_user_identity_number
                            .clone()
                            .map(|id| format!("Mixin ID: {id}"))
                            .unwrap_or_default(),
                    )
                    .text_size(move || appearance.secondary_size()),
                ),
        );
    } else if row.category.ends_with("_TRANSCRIPT") {
        if transcript.is_none()
            && row.media_status == "CANCELED"
            && matches!(row.status.as_str(), "SENDING" | "FAILED")
            && row.sender_id == actions.account.with(|account| account.account_id())
        {
            let id = row.message_id.clone();
            content = content.child(ui::action(
                move || i18n.tr("Retry", "重试"),
                move || {
                    let client = actions.account.with(|account| account.clone());
                    let id = id.clone();
                    spawn(async move {
                        actions.failed.set(
                            runtime::commit(async move {
                                client.attachment().resume_attachment(id, None).await
                            })
                            .await
                            .is_err(),
                        );
                    });
                },
            ));
        }
        let preview_row = row.clone();
        content = content.child(ui::action(
            move || i18n.text(Key::Transcript),
            move || {
                if let Some(root) = transcript.clone() {
                    actions.open_transcript_summary(root, preview_row.message_id.clone());
                } else {
                    actions.open_preview(preview_row.clone());
                }
            },
        ));
        if let Ok(serde_json::Value::Array(previews)) = serde_json::from_str(&row.content) {
            for preview in previews.iter().take(4) {
                let name = preview
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let category = preview
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let text = preview
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                content = content.child(
                    nsh_core::prelude::text(format!(
                        "{name}: {}",
                        presentation::preview(i18n, category, text)
                    ))
                    .text_size(move || appearance.primary_size())
                    .number_of_lines(1),
                );
            }
        }
    } else if row.category.ends_with("_LIVE") {
        let preview_row = row.clone();
        content = content.child(ui::action(
            move || label(i18n, "Watch live", "观看直播"),
            move || actions.open_preview(preview_row.clone()),
        ));
    } else if row.category == "APP_CARD" || row.category == "APP_BUTTON_GROUP" {
        content = content.child(message_detail::card(&row, actions));
    } else if row.category.ends_with("_LOCATION") {
        content = content.child(super::message_location::location(&row, actions));
    } else if row.category == "SYSTEM_SAFE_INSCRIPTION" {
        content = content.child(message_system::inscription(&row, actions));
    } else if row.category == "SYSTEM_ACCOUNT_SNAPSHOT" || row.category == "SYSTEM_SAFE_SNAPSHOT" {
        let snapshot = row.clone();
        content = content.child(
            ui::command()
                .flex_col()
                .items_start()
                .disabled(row.snapshot_id.is_none())
                .on_press(move || actions.open_preview(snapshot.clone()))
                .child(
                    text(format!(
                        "{} {}",
                        row.snapshot_amount.clone().unwrap_or_default(),
                        row.snapshot_asset_symbol.clone().unwrap_or_default()
                    ))
                    .font_weight(FontWeight::MEDIUM),
                ),
        );
        if let Some(memo) = row.snapshot_memo.clone().filter(|s| !s.is_empty()) {
            content = content.child(
                text(memo)
                    .text_size(move || appearance.primary_size())
                    .select_text(),
            );
        }
    } else {
        content = content.child(
            text(presentation::preview(i18n, &row.category, &row.content))
                .text_size(move || appearance.primary_size()),
        );
    }
    if let Some(caption) = row.caption.as_deref().filter(|s| !s.is_empty()) {
        content = content.child(message_text::read(&row, caption, actions, caption_root));
    }
    content.into_element()
}

fn post_preview(content: &str) -> String {
    let mut preview = String::new();
    let mut utf16_units = 0;
    for (index, line) in content.lines().take(10).enumerate() {
        for part in [if index == 0 { "" } else { "\r\n" }, line] {
            for character in part.chars() {
                let units = character.len_utf16();
                if utf16_units + units > 1024 {
                    return preview;
                }
                preview.push(character);
                utf16_units += units;
            }
        }
    }
    preview
}
