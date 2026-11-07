//! Shared presentation for persisted reply quotations and the pending reply bar.
use mixin_desktop_api::MessageListView;
use nsh_base::{text_view::Parser, theme};
use nsh_core::{element::ImageSrc, prelude::*};
use serde_json::Value;

use super::{mentions::MentionNames, message_actions::Actions, message_text};
use crate::{appearance::identity_badge, i18n::Key, presentation, ui};

struct Quote {
    author: String,
    category: String,
    content: String,
    membership: Option<String>,
    artwork: Option<Artwork>,
    valid: bool,
}

enum Artwork {
    Image {
        source: String,
        thumbnail: String,
    },
    Sticker {
        id: Option<String>,
        source: String,
        kind: String,
    },
    Contact {
        name: String,
        avatar: String,
    },
}

impl Quote {
    fn selected(row: &MessageListView) -> Self {
        let value = serde_json::json!({
            "type": row.category, "status": row.status, "content": row.content, "user_full_name": row.sender_name,
            "thumb_image": row.thumb_image, "thumb_url": row.thumb_url,
            "media_url": row.media_url, "media_name": row.media_name,
            "asset_url": row.sticker_asset_url, "asset_type": row.sticker_asset_type,
            "sticker_id": row.sticker_id,
            "shared_user_full_name": row.shared_user_full_name,
            "shared_user_identity_number": row.shared_user_identity_number,
            "shared_user_avatar_url": row.shared_user_avatar_url,
        });
        Self::parse(Some(value), row.sender_membership.clone())
    }

    fn parse(value: Option<Value>, membership: Option<String>) -> Self {
        let value = value.unwrap_or(Value::Null);
        let string = |key| {
            value
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let category = string("type");
        let valid = ([
            "_TEXT",
            "_IMAGE",
            "_VIDEO",
            "_AUDIO",
            "_DATA",
            "_STICKER",
            "_CONTACT",
            "_LOCATION",
            "_POST",
            "_TRANSCRIPT",
            "_LIVE",
        ]
        .iter()
        .any(|suffix| category.ends_with(suffix))
            || matches!(category.as_str(), "APP_CARD" | "APP_BUTTON_GROUP"))
            && super::message_system::supported(&category)
            && !matches!(string("status").as_str(), "UNKNOWN" | "FAILED");
        let artwork = if !valid {
            None
        } else if category.ends_with("_IMAGE")
            || category.ends_with("_VIDEO")
            || category.ends_with("_LIVE")
        {
            Some(Artwork::Image {
                source: if category.ends_with("_IMAGE") {
                    string("media_url")
                } else if category.ends_with("_LIVE") {
                    string("thumb_url")
                } else {
                    String::new()
                },
                thumbnail: string("thumb_image"),
            })
        } else if category.ends_with("_STICKER") {
            Some(Artwork::Sticker {
                id: value
                    .get("sticker_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                source: string("asset_url"),
                kind: string("asset_type"),
            })
        } else if category.ends_with("_CONTACT") {
            Some(Artwork::Contact {
                name: string("shared_user_full_name"),
                avatar: string("shared_user_avatar_url"),
            })
        } else {
            None
        };
        let raw = string("content");
        let content = if !valid {
            String::new()
        } else if category.ends_with("_DATA") {
            string("media_name")
        } else if category.ends_with("_CONTACT") {
            string("shared_user_identity_number")
        } else if category.ends_with("_POST") {
            let bounded: String = raw
                .lines()
                .take(10)
                .collect::<Vec<_>>()
                .join("\n")
                .chars()
                .take(1024)
                .collect();
            Parser::new()
                .markdown(&bounded)
                .plain_text()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        } else if category == "APP_CARD" {
            serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|card| card["title"].as_str().map(str::to_owned))
                .unwrap_or_default()
        } else if category == "APP_BUTTON_GROUP" {
            serde_json::from_str::<Vec<Value>>(&raw)
                .unwrap_or_default()
                .iter()
                .filter_map(|action| action["label"].as_str())
                .map(|label| format!("[{label}]"))
                .collect()
        } else {
            raw
        };
        Self {
            author: string("user_full_name"),
            category,
            content,
            membership,
            artwork,
            valid,
        }
    }
}

pub(super) fn quoted(
    row: &MessageListView,
    actions: Actions,
    target: Option<Signal<Option<String>>>,
    transcript: bool,
) -> AnyElement {
    let Some(id) = row.quote_message_id.clone().filter(|id| !id.is_empty()) else {
        return view().hidden().into_element();
    };
    let quote = Quote::parse(
        row.quote_content
            .as_deref()
            .and_then(|content| serde_json::from_str(content).ok()),
        row.quote_user_membership.clone(),
    );
    let valid = quote.valid;
    let card = card(quote, actions, false);
    if !valid {
        return card.into_element();
    }
    let navigation =
        use_context::<crate::navigation::ChatNavigation>().expect("Home provides chat navigation");
    let conversation = row.conversation_id.clone();
    let origin = (!transcript).then(|| row.message_id.clone());
    ui::command()
        .p_0()
        .min_w_0()
        .min_h_0()
        .justify_start()
        .accessibility_label(move || Some(actions.i18n.tr("Locate in chat", "在聊天中查看").into()))
        .on_press(move || {
            actions.close_preview();
            if let Some(target) = target {
                batch(|| {
                    navigation.cancel_pending();
                    navigation.panel.set(None);
                    navigation.return_target.set(origin.clone());
                    target.set(Some(id.clone()));
                });
            } else if !navigation.jump_in_selected(&conversation, id.clone(), origin.clone()) {
                navigation.open_conversation(
                    actions.account.with(|account| account.clone()),
                    conversation.clone(),
                    Some(id.clone()),
                    actions.failed,
                );
            }
        })
        .child(card)
        .into_element()
}

pub(super) fn reply_bar(actions: Actions) -> impl IntoElement {
    when(
        move || actions.quote.with(Option::is_some),
        move || {
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .child(dynamic(move || {
                    actions.quote.with(|row| {
                        row.as_ref()
                            .map(|row| {
                                let id = row.message_id.clone();
                                let conversation = row.conversation_id.clone();
                                let navigation = use_context::<crate::navigation::ChatNavigation>()
                                    .expect("Home provides chat navigation");
                                ui::command()
                                    .p_0()
                                    .min_h_0()
                                    .min_w_0()
                                    .flex_1()
                                    .justify_start()
                                    .accessibility_label(move || {
                                        Some(
                                            actions
                                                .i18n
                                                .tr("Locate in chat", "在聊天中查看")
                                                .into(),
                                        )
                                    })
                                    .on_press(move || {
                                        navigation.jump_in_selected(
                                            &conversation,
                                            id.clone(),
                                            None,
                                        );
                                    })
                                    .child(card(Quote::selected(row), actions, true))
                                    .into_element()
                            })
                            .unwrap_or_else(|| view().hidden().into_element())
                    })
                }))
                .child(ui::action(
                    move || actions.i18n.text(Key::Cancel),
                    move || actions.quote.set(None),
                ))
        },
    )
}

fn card(quote: Quote, actions: Actions, input: bool) -> View {
    let appearance =
        use_context::<crate::appearance::Appearance>().expect("Home provides chat appearance");
    let i18n = actions.i18n;
    let names = use_context::<MentionNames>();
    let quote = stored(quote);
    view()
        .min_h(px(50.0))
        .min_w_0()
        .flex_1()
        .flex_row()
        .items_center()
        .gap_2()
        .style_when(move || !input, |style| style.rounded_lg())
        .overflow_hidden()
        .bg(move || theme().foreground.opacity(if input { 0.0 } else { 0.04 }))
        .border_l(px(6.0))
        .border_color(move || theme().primary)
        .child(
            view()
                .flex_1()
                .min_w_0()
                .py_1()
                .pl_2()
                .gap_1()
                .child(when(
                    move || quote.with(|quote| quote.valid && !quote.author.is_empty()),
                    move || {
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .min_w_0()
                            .child(
                                text(quote.with(|quote| quote.author.clone()))
                                    .number_of_lines(1)
                                    .text_size(move || appearance.secondary_size())
                                    .text_color(move || theme().primary),
                            )
                            .child(quote.with(|quote| {
                                identity_badge(false, false, false, quote.membership.as_deref())
                            }))
                    },
                ))
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .min_w_0()
                        .child(quote.with(|quote| category_icon(&quote.category)))
                        .child(
                            text(move || {
                                quote.with(|quote| {
                                    let text = if !quote.valid {
                                        if quote.category.is_empty()
                                            || quote.category == "MESSAGE_RECALL"
                                        {
                                            i18n.tr("Message not found", "消息不存在").to_owned()
                                        } else {
                                            i18n.tr(
                                                "This message type is not supported.",
                                                "暂不支持此消息类型。",
                                            )
                                            .to_owned()
                                        }
                                    } else if quote.category.ends_with("_TEXT") {
                                        names
                                            .map(|names| {
                                                names.0.with(|names| {
                                                    message_text::document(
                                                        &quote.content,
                                                        names,
                                                        "",
                                                    )
                                                    .plain_text()
                                                })
                                            })
                                            .unwrap_or_else(|| quote.content.clone())
                                    } else if quote.category.ends_with("_LIVE") {
                                        i18n.tr("Live", "直播").to_owned()
                                    } else if quote.category.ends_with("_AUDIO") {
                                        i18n.tr("Audio", "语音").to_owned()
                                    } else if quote.category.ends_with("_POST")
                                        || quote.category.ends_with("_DATA")
                                        || quote.category.ends_with("_CONTACT")
                                        || quote.category.starts_with("APP_")
                                    {
                                        if quote.content.is_empty() {
                                            if quote.category.starts_with("APP_") {
                                                i18n.text(Key::Bots).to_owned()
                                            } else {
                                                presentation::preview(i18n, &quote.category, "")
                                            }
                                        } else {
                                            quote.content.clone()
                                        }
                                    } else {
                                        presentation::preview(i18n, &quote.category, "")
                                    };
                                    let mut lines = text.lines();
                                    let first = lines.next().unwrap_or_default();
                                    if lines.next().is_some() {
                                        format!("{first}…")
                                    } else {
                                        first.to_owned()
                                    }
                                })
                            })
                            .min_w_0()
                            .flex_1()
                            .number_of_lines(1)
                            .text_size(move || appearance.tertiary_size())
                            .text_color(move || theme().muted_foreground),
                        ),
                ),
        )
        .child(quote.with(|quote| match &quote.artwork {
            Some(Artwork::Image { source, thumbnail }) => {
                thumbnail_image(source.clone(), thumbnail.clone()).into_element()
            }
            Some(Artwork::Sticker { id, source, kind }) => {
                let id = id.clone();
                let source = source.clone();
                let kind = kind.clone();
                crate::sticker::asset(
                    actions.account.with(|account| account.clone()),
                    move || (id.clone(), source.clone(), kind.clone()),
                    48.0,
                )
                .into_element()
            }
            Some(Artwork::Contact { name, avatar }) => {
                let name = name.clone();
                let avatar = avatar.clone();
                view()
                    .size_12()
                    .items_center()
                    .justify_center()
                    .child(
                        ui::avatar_view(move || name.clone().into(), move || avatar.clone())
                            .size_9(),
                    )
                    .into_element()
            }
            _ => view().hidden().into_element(),
        }))
}

fn thumbnail_image(source: String, data: String) -> impl IntoElement {
    let failed = signal(source.is_empty());
    let decoded = super::message_media::thumbnail(move || (!data.is_empty()).then(|| data.clone()));
    let source = stored(source);
    view()
        .size_12()
        .flex_shrink_0()
        .rounded(move || theme().radius)
        .overflow_hidden()
        .bg(move || theme().muted)
        .child(dynamic(move || {
            let uri = if failed.get() {
                decoded().map(|uri| uri.to_string()).unwrap_or_default()
            } else {
                source.with(|source| source.clone())
            };
            if uri.is_empty() {
                view()
                    .size_12()
                    .items_center()
                    .justify_center()
                    .child(ui::icon(ui::Icon::Chat).text_color(move || theme().muted_foreground))
                    .into_element()
            } else {
                image(uri)
                    .size_12()
                    .object_fit(ResizeMode::Cover)
                    .on_error(move |_: &str| failed.set(true))
                    .into_element()
            }
        }))
}

fn category_icon(category: &str) -> AnyElement {
    let source = if category.ends_with("_TEXT") {
        return view().hidden().into_element();
    } else if category.ends_with("_STICKER") {
        return ui::icon(ui::Icon::Sticker)
            .size(px(14.0))
            .text_color(move || theme().muted_foreground)
            .into_element();
    } else if category.ends_with("_IMAGE") {
        include_str!("../../assets/quote/image.svg")
    } else if category.ends_with("_VIDEO") {
        include_str!("../../assets/quote/video.svg")
    } else if category.ends_with("_AUDIO") {
        include_str!("../../assets/quote/audio.svg")
    } else if category.ends_with("_LOCATION") {
        include_str!("../../assets/quote/location.svg")
    } else if category.ends_with("_LIVE") {
        include_str!("../../assets/quote/live.svg")
    } else if category.ends_with("_CONTACT") {
        include_str!("../../assets/quote/contact.svg")
    } else if category.starts_with("APP_") {
        include_str!("../../assets/quote/app_button.svg")
    } else if ["_DATA", "_POST", "_TRANSCRIPT"]
        .iter()
        .any(|suffix| category.ends_with(suffix))
    {
        include_str!("../../assets/quote/file.svg")
    } else {
        include_str!("../../assets/quote/recall.svg")
    };
    svg(ImageSrc::svg_markup(source))
        .size(px(14.0))
        .flex_shrink_0()
        .text_color(move || theme().muted_foreground)
        .into_element()
}
