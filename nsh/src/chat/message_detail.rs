use mixin_desktop_api::MessageListView;
use nsh_core::{LineHeight, prelude::*};
use serde_json::Value;

use super::{
    message_actions::{Actions, Detail},
    message_appearance::bubble_surface,
    message_links,
};
use crate::{
    appearance::Appearance,
    i18n::Key,
    runtime,
    ui::{self, Typography},
};

fn appearance() -> Appearance {
    use_context::<Appearance>().expect("Home provides chat appearance")
}

/// Whether an `APP_CARD` holds a title and its own actions, not one action of its own.
pub(super) fn actions_card(content: &str) -> bool {
    serde_json::from_str::<Value>(content)
        .is_ok_and(|card| card["action"].as_str().is_none_or(str::is_empty))
}

pub(super) fn card(row: &MessageListView, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let Ok(value) = serde_json::from_str::<Value>(&row.content) else {
        return text(move || i18n.tr("Invalid card", "无法读取卡片"))
            .text_role(ui::TextRole::Body)
            .into_element();
    };
    if row.category == "APP_BUTTON_GROUP" {
        return commands(row, value.as_array().cloned().unwrap_or_default(), actions)
            .min_w(ui::action_group_min_width())
            .max_w(ui::action_group_max_width())
            .into_element();
    }
    match value["action"].as_str().filter(|action| !action.is_empty()) {
        Some(action) => app_card(row, action.to_owned(), &value, actions),
        None => titled_card(row, &value, actions),
    }
}

fn app_card(row: &MessageListView, action: String, value: &Value, actions: Actions) -> AnyElement {
    let appearance = appearance();
    let string = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
    let description = string("description");
    let description = description.lines().next().unwrap_or_default().to_owned();
    let input = action.starts_with("input:");
    ui::command()
        .justify_start()
        .gap_2()
        .p_0()
        .min_h_0()
        .min_w_0()
        .disabled(move || input && (!actions.can_send.get() || actions.busy.get()))
        .on_press(message_links::card_action(
            actions,
            row,
            action,
            Some(value.clone()),
        ))
        .child(
            image(string("icon_url"))
                .size_10()
                .flex_shrink_0()
                .rounded(ui::radius_app_icon())
                .object_fit(ResizeMode::Cover),
        )
        .child(
            view()
                .min_w_0()
                .flex_shrink(1.0)
                .child(
                    text(string("title"))
                        .number_of_lines(1)
                        .text_size(move || appearance.secondary_size()),
                )
                .child(
                    text(description)
                        .number_of_lines(1)
                        .text_size(move || appearance.tertiary_size())
                        .text_color(ui::message_secondary),
                ),
        )
        .into_element()
}

/// A card with its own surface, and its actions under it, outside the bubble.
fn titled_card(row: &MessageListView, value: &Value, actions: Actions) -> AnyElement {
    let appearance = appearance();
    let region = super::message_media::region();
    let mine = row.sender_id == actions.account.with(|account| account.account_id());
    let string = |key: &str| value[key].as_str().unwrap_or_default().to_owned();
    let cover = &value["cover"];
    let (source, aspect) = match cover["url"].as_str().filter(|url| !url.is_empty()) {
        Some(url) => {
            let side = |key: &str| cover[key].as_f64().filter(|side| *side > 0.0);
            let aspect = side("width").zip(side("height"));
            let aspect = aspect.map_or(1.5, |(width, height)| (width / height).max(1.5));
            (url.to_owned(), aspect as f32)
        }
        None => (string("cover_url"), 1.0),
    };
    let mut surface = view()
        .rounded(ui::radius_md())
        .overflow_hidden()
        .pb(ui::chip_inset());
    if !source.is_empty() {
        surface = surface.child(
            image(source)
                .w_full()
                .aspect_ratio(aspect)
                .object_fit(ResizeMode::Cover),
        );
    }
    surface = surface.child(
        view()
            .pt(ui::chip_inset())
            .px(ui::space_md())
            .gap_2()
            .child(
                text(string("title"))
                    .text_size(move || appearance.primary_size())
                    .text_emphasis()
                    .select_text(),
            )
            .child(
                text(string("description"))
                    .text_size(move || appearance.primary_size())
                    .select_text(),
            ),
    );
    let least = ui::actions_card_min_width().0;
    let most = ui::actions_card_max_width().0;
    view()
        .w(move || px((region() * 0.5).clamp(least, most)))
        .max_w(relative(1.0))
        .gap_2()
        .child(
            view()
                .rounded(ui::radius_md())
                .bg(move || bubble_surface(mine))
                .shadow_xs()
                .shadow_color(ui::bubble_shadow())
                .child(surface),
        )
        .child(commands(
            row,
            value["actions"].as_array().cloned().unwrap_or_default(),
            actions,
        ))
        .into_element()
}

fn commands(row: &MessageListView, commands: Vec<Value>, actions: Actions) -> View {
    let appearance = appearance();
    let mut group = view().flex_row().flex_wrap().gap_2();
    for command in commands {
        let Some(action) = command["action"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        let action = action.to_owned();
        let title = command["label"].as_str().unwrap_or_default().to_owned();
        let color = command["color"]
            .as_str()
            .map(|color| color.trim_start_matches('#'))
            .filter(|color| color.len() == 6)
            .and_then(|color| u32::from_str_radix(color, 16).ok())
            .map_or_else(ui::fg_primary, |color| rgb(color).into());
        let input = action.starts_with("input:");
        let on_press = message_links::card_action(actions, row, action, None);
        // A state fill replaces a button's own, so the plate sits on a parent.
        group = group.child(
            view()
                .rounded(ui::radius_md())
                .bg(ui::surface_list)
                .shadow_xs()
                .shadow_color(ui::bubble_shadow())
                .child(
                    ui::command()
                        .min_h_0()
                        .px(ui::action_button_inset_x())
                        .py_2()
                        .rounded(ui::radius_md())
                        .disabled(move || input && (!actions.can_send.get() || actions.busy.get()))
                        .on_press(on_press)
                        .child(
                            text(title)
                                .text_size(move || appearance.secondary_size())
                                .line_height(LineHeight::Multiple(1.0))
                                .text_color(move || color),
                        ),
                ),
        );
    }
    group
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
            ui::dialog(
                presence,
                move || actions.close_preview(),
                view()
                    .bg(ui::surface_pane)
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .gap_3()
                    .style_when(qr, |style| {
                        style
                            .flex_auto()
                            .flex_grow(0.0)
                            .w_96()
                            .max_w(relative(1.0))
                            .max_h(relative(0.9))
                            .p_4()
                            .rounded(ui::radius_lg())
                            .shadow_lg()
                    })
                    .style_when(user_profile, |style| {
                        ui::dialog_surface(style, ui::DialogWidth::Compact)
                            .flex_auto()
                            .flex_grow(0.0)
                            .p_0()
                            .gap_0()
                            .overflow_hidden()
                    })
                    .child(
                        view()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .style_when(qr, |style| style.items_center())
                            .style_when(user_profile, |style| style.hidden())
                            .child(when(qr, move || {
                                ui::dialog_title(move || i18n.tr("QR code", "二维码"))
                                    .flex_1()
                                    .min_w_0()
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
                        ui::error_text(move || i18n.text(Key::CommandFailed))
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
                        .text_role(ui::TextRole::Body)
                        .select_text()
                        .style_when(user_profile, |style| style.hidden()),
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
        .child(ui::dialog_title(move || i18n.tr("Add sticker", "添加贴纸")))
        .child(
            image(row.media_url.unwrap_or_default())
                .size_96()
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
