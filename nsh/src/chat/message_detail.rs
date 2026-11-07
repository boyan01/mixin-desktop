use mixin_desktop_api::MessageListView;
use nsh_base::theme;
use nsh_core::prelude::*;
use serde_json::Value;

use super::{
    message_actions::{Actions, Detail},
    message_links,
};
use crate::{i18n::Key, runtime, ui};

pub(super) fn card(row: &MessageListView, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let Ok(value) = serde_json::from_str::<Value>(&row.content) else {
        return text(move || i18n.tr("Invalid card", "无法读取卡片"))
            .text_sm()
            .into_element();
    };
    let string = |key| {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut card = view().w_64().max_w(relative(1.0)).gap_2();
    let cover = value
        .get("cover")
        .and_then(|c| c.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let cover = if cover.is_empty() {
        string("cover_url")
    } else {
        cover.to_owned()
    };
    if !cover.is_empty() {
        card = card.child(image(cover).w_full().h_32().object_fit(ResizeMode::Cover));
    }
    card = card
        .child(
            text(string("title"))
                .font_weight(FontWeight::SEMIBOLD)
                .select_text(),
        )
        .child(text(string("description")).text_sm().select_text());
    let primary = string("action");
    let single_action = row.category != "APP_BUTTON_GROUP" && !primary.is_empty();
    let commands = if row.category == "APP_BUTTON_GROUP" {
        value.as_array().cloned().unwrap_or_default()
    } else if single_action {
        vec![serde_json::json!({"label": i18n.tr("Open", "打开"), "action": primary})]
    } else {
        value
            .get("actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for command in commands {
        let Some(action) = command
            .get("action")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let action = action.to_owned();
        let title = command
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let input = action.starts_with("input:");
        let on_press =
            message_links::card_action(actions, row, action, single_action.then(|| value.clone()));
        card = card.child(
            ui::command()
                .justify_start()
                .disabled(move || input && (!actions.can_send.get() || actions.busy.get()))
                .on_press(on_press)
                .child(text(title).text_sm()),
        );
    }
    card.into_element()
}

pub(super) fn preview(actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let presence = crate::motion::Presence::modal(move || actions.preview.with(Option::is_some));
    let shown = signal_with(actions.preview.get_untracked(), SignalEquals::Never);
    effect(move || {
        if let Some(detail) = actions.preview.get() {
            shown.set(Some(detail));
        }
    });
    let qr = move || shown.with(|detail| matches!(detail, Some(Detail::Qr(_))));
    let user_profile = move || shown.with(|detail| matches!(detail, Some(Detail::User(_))));
    when(
        move || presence.mounted(),
        move || {
            ui::modal(
                presence,
                move || actions.close_preview(),
                view()
                    .bg(move || theme().background)
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .gap_3()
                    .style_when(qr, |style| {
                        style
                            .flex_auto()
                            .flex_grow(0.0)
                            .w(px(360.0))
                            .max_w(relative(1.0))
                            .max_h(relative(0.9))
                            .p_4()
                            .rounded_xl()
                            .shadow_lg()
                    })
                    .style_when(user_profile, |style| {
                        style
                            .flex_auto()
                            .flex_grow(0.0)
                            .w(px(340.0))
                            .max_w(relative(1.0))
                            .h(px(560.0))
                            .max_h(relative(0.9))
                            .rounded_xl()
                            .overflow_hidden()
                    })
                    .child(
                        view()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .style_when(qr, |style| style.items_center())
                            .style_when(user_profile, |style| style.flex_none())
                            .child(when(qr, move || {
                                text(move || i18n.tr("QR code", "二维码"))
                                    .flex_1()
                                    .min_w_0()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                            }))
                            .child(when(
                                move || !actions.preview_history.with(Vec::is_empty),
                                move || {
                                    ui::action(
                                        move || i18n.text(Key::Back),
                                        move || actions.back_preview(),
                                    )
                                },
                            ))
                            .child(ui::action(
                                move || i18n.tr("Close", "关闭"),
                                move || actions.close_preview(),
                            )),
                    )
                    .child(dynamic(move || {
                        let row = shown.get();
                        untracked(|| match row {
                            Some(Detail::Qr(content)) => {
                                super::message_qr::screen(content).into_element()
                            }
                            Some(Detail::Message(row)) if row.category.ends_with("_TRANSCRIPT") => {
                                super::message_transcript::screen(row.message_id, actions)
                                    .into_element()
                            }
                            Some(Detail::TranscriptSummary {
                                transcript_id,
                                child_message_id,
                            }) => super::message_transcript::summary(
                                transcript_id,
                                child_message_id,
                                actions,
                            )
                            .into_element(),
                            Some(Detail::Message(row)) if row.category.ends_with("_POST") => {
                                super::message_post::reader(
                                    row.content,
                                    row.conversation_id,
                                    actions,
                                )
                                .into_element()
                            }
                            Some(Detail::Markdown {
                                message,
                                transcript,
                            }) => super::message_post::attachment(*message, transcript, actions)
                                .into_element(),
                            Some(Detail::Message(row))
                                if row.category.ends_with("_VIDEO")
                                    || row.category.ends_with("_LIVE") =>
                            {
                                let source = if row.category.ends_with("_LIVE") {
                                    serde_json::from_str::<Value>(&row.content)
                                        .ok()
                                        .and_then(|v| {
                                            v.get("url").and_then(Value::as_str).map(str::to_owned)
                                        })
                                        .unwrap_or_default()
                                } else {
                                    row.media_url.unwrap_or_default()
                                };
                                video(source)
                                    .paused(move || !presence.interactive())
                                    .controls(true)
                                    .flex_1()
                                    .w_full()
                                    .min_h_0()
                                    .on_error(move |_: &str| actions.failed.set(true))
                                    .into_element()
                            }
                            Some(Detail::Image {
                                message,
                                transcript,
                            }) => {
                                super::gallery::screen(*message, transcript, actions).into_element()
                            }
                            Some(Detail::AddSticker(row)) => {
                                add_image_sticker(*row, actions).into_element()
                            }
                            Some(Detail::Sticker(sticker_id)) => crate::composer::sticker_detail(
                                actions.account.with(|value| value.clone()),
                                sticker_id,
                            )
                            .into_element(),
                            Some(Detail::User(id)) => crate::conversation::profile(
                                actions.account.with(|a| a.clone()),
                                id,
                                move || {
                                    if actions.preview_history.with_untracked(Vec::is_empty) {
                                        actions.close_preview();
                                    } else {
                                        actions.back_preview();
                                    }
                                },
                            )
                            .into_element(),
                            Some(Detail::Message(row)) if row.snapshot_id.is_some() => {
                                super::message_snapshot::screen(*row, actions).into_element()
                            }
                            Some(Detail::Message(row)) => image(row.media_url.unwrap_or_default())
                                .flex_1()
                                .min_h_0()
                                .w_full()
                                .object_fit(ResizeMode::Contain)
                                .label(move || i18n.text(Key::Photo))
                                .into_element(),
                            None => view().hidden().into_element(),
                        })
                    }))
                    .child(when(actions.failed, move || {
                        text(move || i18n.text(Key::CommandFailed))
                            .text_color(move || theme().destructive)
                    }))
                    .child(
                        text(move || {
                            shown.with(|m| {
                                m.as_ref()
                                    .and_then(|detail| match detail {
                                        Detail::Message(m) => m.caption.clone(),
                                        Detail::User(_)
                                        | Detail::Qr(_)
                                        | Detail::AddSticker(_)
                                        | Detail::Sticker(_)
                                        | Detail::Markdown { .. }
                                        | Detail::TranscriptSummary { .. } => None,
                                        Detail::Image { .. } => None,
                                    })
                                    .unwrap_or_default()
                            })
                        })
                        .text_sm()
                        .select_text(),
                    ),
            )
        },
    )
}

fn add_image_sticker(row: MessageListView, actions: Actions) -> impl IntoElement {
    let i18n = actions.i18n;
    let id = stored(row.message_id);
    let active = move || {
        actions.preview.with(|detail| {
            matches!(detail, Some(Detail::AddSticker(row)) if id.with(|id| *id == row.message_id))
        })
    };
    view()
        .flex_1()
        .w_full()
        .min_h_0()
        .items_center()
        .justify_center()
        .gap_6()
        .child(
            text(move || i18n.tr("Add sticker", "添加贴纸"))
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD),
        )
        .child(
            image(row.media_url.unwrap_or_default())
                .size(px(400.0))
                .max_w(relative(0.9))
                .max_h(relative(0.7))
                .object_fit(ResizeMode::Contain)
                .label(move || i18n.text(Key::Photo))
                .on_error(move |_: &str| actions.failed.set(true)),
        )
        .child(
            ui::action(
                move || i18n.tr("Save", "保存"),
                move || {
                    if !active() || actions.busy.get_untracked() {
                        return;
                    }
                    let client = actions.account.with(|account| account.clone());
                    let id = id.with(|id| id.clone());
                    actions.busy.set(true);
                    actions.failed.set(false);
                    actions.task(async move {
                        let result = runtime::commit(async move {
                            client.sticker().add_sticker_from_file(id).await
                        })
                        .await;
                        actions.failed.set(result.is_err());
                        actions.busy.set(false);
                        if result.is_ok() && active() {
                            if actions.preview_history.with_untracked(Vec::is_empty) {
                                actions.close_preview();
                            } else {
                                actions.back_preview();
                            }
                        }
                    });
                },
            )
            .disabled(move || actions.busy.get() || !active())
            .child(ui::busy_indicator(
                move || actions.busy.get(),
                move || i18n.tr("Saving…", "正在保存…").into(),
            )),
        )
}
