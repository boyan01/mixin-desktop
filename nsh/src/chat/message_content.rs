use super::{
    message_actions::Actions,
    message_appearance::{self, Frame, Inset, Stamp},
    message_detail,
    message_media::Transfer,
    message_system, message_text,
};
use crate::{
    appearance::{Appearance, identity_badge},
    i18n::{I18n, Key},
    presentation,
    ui::{self, Typography},
};
use mixin_desktop_api::MessageListView;
use nsh_base::text_view;
use nsh_core::{LineHeight, element::ImageSrc, prelude::*};

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

fn appearance() -> Appearance {
    use_context::<Appearance>().expect("Home provides chat appearance")
}

/// Where a body is shown. A timeline row passes a live delivery status so that
/// status changes do not rebuild its media.
#[derive(Clone, Default)]
pub(super) struct Placement {
    pub target: Option<Signal<Option<String>>>,
    pub transcript: Option<String>,
    pub status: Option<Memo<String>>,
    /// The body fills a timeline bubble, which clips it and hosts its status.
    pub framed: bool,
}

pub(super) fn body(row: MessageListView, actions: Actions, placement: Placement) -> AnyElement {
    let framed = placement.framed;
    let frame = Frame::of(&row);
    if let Some(unavailable) = message_system::unavailable(&row, actions) {
        return if framed {
            frame.padded(view().child(unavailable)).into_element()
        } else {
            unavailable
        };
    }
    if message_system::centered(&row.category) {
        return message_system::notice(&row, actions, placement.target);
    }
    let appearance = appearance();
    let transcript = placement.transcript.clone();
    let mine = row.sender_id == actions.account.with(|account| account.account_id());
    // A status at the end of the text needs room there, not just over it.
    let room = (framed && frame.stamp == Stamp::Inline)
        .then(|| message_appearance::stamp_reserve(&row, mine && frame.delivery));
    let mut content = view()
        .min_w_0()
        .text_size(move || appearance.primary_size());
    if !framed {
        content = content.gap_2();
    }
    if row
        .quote_message_id
        .as_ref()
        .is_some_and(|id| !id.is_empty())
    {
        let quote = view().child(super::message_quote::quoted(
            &row,
            actions,
            placement.target,
            transcript.is_some(),
        ));
        // Flutter holds a quote to the picture under it.
        content = content.child(if matches!(frame.inset, Inset::Flush | Inset::Caption) {
            quote.w_0().min_w(relative(1.0))
        } else {
            quote.min_w_0()
        });
    }
    let mut main = view()
        .min_w_0()
        .child(kind_body(&row, actions, placement, room));
    if !framed {
        main = main.gap_2();
    }
    if let Some(caption) = row.caption.as_deref().filter(|s| !s.is_empty()) {
        let caption = message_text::read(&row, caption, actions, transcript, room);
        main = main.child(if framed {
            // Wraps at the picture's width rather than widening the bubble.
            view()
                .w_0()
                .min_w(relative(1.0))
                .px(ui::space_md())
                .py(ui::chip_inset())
                .child(caption)
        } else {
            view().min_w_0().child(caption)
        });
    }
    content
        .child(if framed { frame.padded(main) } else { main })
        .into_element()
}

fn kind_body(
    row: &MessageListView,
    actions: Actions,
    placement: Placement,
    room: Option<f32>,
) -> AnyElement {
    let category = row.category.as_str();
    let Placement {
        transcript,
        status,
        framed,
        ..
    } = placement;
    let mine = row.sender_id == actions.account.with(|account| account.account_id());
    let delivery = status.filter(|_| mine);
    if category.ends_with("_TEXT") {
        message_text::read(row, &row.content, actions, transcript, room).into_element()
    } else if category == "MESSAGE_RECALL" {
        recalled(row, actions)
    } else if category.ends_with("_POST") {
        post(
            row,
            actions,
            framed.then(|| message_appearance::post_stamp(row, delivery)),
        )
    } else if category.ends_with("_STICKER") {
        sticker(row, actions)
    } else if category.ends_with("_IMAGE") || category.ends_with("_VIDEO") {
        super::message_media::card(row.clone(), transcript, actions, status, framed).into_element()
    } else if category.ends_with("_AUDIO") && available(row) {
        super::audio::body(row.clone(), transcript, actions).into_element()
    } else if is_attachment(row) {
        file(row.clone(), transcript, actions)
    } else if category.ends_with("_CONTACT") {
        contact(row, actions)
    } else if category.ends_with("_TRANSCRIPT") {
        transcript_card(
            row,
            actions,
            transcript,
            framed.then(|| message_appearance::media_stamp(row, delivery)),
        )
    } else if category.ends_with("_LIVE") {
        let preview = row.clone();
        ui::action(
            move || actions.i18n.tr("Watch live", "观看直播"),
            move || actions.open_preview(preview.clone()),
        )
        .into_element()
    } else if category == "APP_CARD" || category == "APP_BUTTON_GROUP" {
        message_detail::card(row, actions)
    } else if category.ends_with("_LOCATION") {
        super::message_location::location(row, actions)
    } else if category == "SYSTEM_SAFE_INSCRIPTION" {
        message_system::inscription(row, actions).into_element()
    } else if category == "SYSTEM_ACCOUNT_SNAPSHOT" {
        account_snapshot(row, actions)
    } else if category == "SYSTEM_SAFE_SNAPSHOT" {
        safe_snapshot(row, actions)
    } else {
        text(presentation::preview(actions.i18n, category, &row.content)).into_element()
    }
}

fn recalled(row: &MessageListView, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let reedit = use_context::<super::reedit::Reedit>().expect("Home provides recall editing");
    let mine = actions.account.with(|account| account.account_id()) == row.sender_id;
    let id = stored(row.message_id.clone());
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap_1()
        .child(
            svg(ImageSrc::svg_markup(include_str!(
                "../../assets/quote/recall.svg"
            )))
            .size(ui::icon_sm())
            .flex_shrink_0()
            .text_color(ui::message_secondary)
            .accessibility_elements_hidden(true),
        )
        .child(text(move || {
            if mine {
                i18n.tr("You deleted this message", "你删除了这条消息")
            } else {
                i18n.tr("This message was deleted", "这条消息已被删除")
            }
        }))
        .child(when(
            move || mine && id.with(|id| reedit.available(id)),
            move || {
                ui::link(
                    move || i18n.tr("Re-edit", "重新编辑"),
                    move || id.with(|id| reedit.request(id)),
                )
            },
        ))
        .into_element()
}

fn post(row: &MessageListView, actions: Actions, stamp: Option<View>) -> AnyElement {
    let i18n = actions.i18n;
    let appearance = appearance();
    let preview = row.clone();
    let bound = ui::post_max_width();
    let mut card = ui::command()
        .relative()
        .p_0()
        .min_h_12()
        .min_w_0()
        .w(bound)
        .max_w(relative(1.0))
        .flex_col()
        .items_stretch()
        .max_h(bound)
        .overflow_hidden()
        .accessibility_label(move || Some(i18n.text(Key::Post).into()))
        .on_press(move || actions.open_preview(preview.clone()))
        .child(
            view()
                .w_full()
                .min_w_0()
                .min_h_0()
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
            message_appearance::detail_icon()
                .absolute()
                .top_0()
                .right_0(),
        );
    if let Some(stamp) = stamp {
        card = card.child(stamp.absolute().bottom_0().right_0());
    }
    card.into_element()
}

fn sticker(row: &MessageListView, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let id = row.sticker_id.clone().filter(|id| !id.is_empty());
    let url = row.sticker_asset_url.clone().unwrap_or_default();
    if id.is_none() && url.is_empty() {
        return text(move || i18n.text(Key::Sticker)).into_element();
    }
    let kind = row.sticker_asset_type.clone().unwrap_or_default();
    let pressed = id.clone().unwrap_or_default();
    let frame = sticker_size(row.sticker_asset_width, row.sticker_asset_height);
    ui::command()
        .p_0()
        .min_h_0()
        .disabled(id.is_none())
        .accessibility_label(move || Some(i18n.text(Key::Sticker).into()))
        .on_press(move || actions.open_sticker(pressed.clone()))
        .child(crate::sticker::framed_asset(
            actions.account.with(|value| value.clone()),
            move || (id.clone(), url.clone(), kind.clone()),
            frame,
        ))
        .into_element()
}

/// Flutter's `_calculateSize`: an asset's own pixels on this display, within the
/// bounds of a sticker's sides.
fn sticker_size(width: Option<i32>, height: Option<i32>) -> (f32, f32) {
    let ratio = pixel_ratio();
    let least = ui::sticker_min_size().0 * ratio;
    let most = ui::sticker_max_size().0 * ratio;
    let (width, height) = match (width, height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => (width as f32, height as f32),
        _ => (most, most),
    };
    let (width, height) = if width < least {
        let scale = least / width;
        ((scale * width).min(most), scale * height)
    } else if height < least {
        let scale = least / height;
        (scale * width, (scale * height).min(most))
    } else if width > most || height > most {
        let scale = most / width.max(height);
        (scale * width, scale * height)
    } else {
        (width, height)
    };
    (width / ratio, height / ratio)
}

/// One pressable card for a file in any state: the press opens, transfers or
/// retries it, and the glyph and second line say which. Undownloaded audio uses
/// it too, until its player has a file to play.
fn file(row: MessageListView, transcript: Option<String>, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let appearance = appearance();
    let done = available(&row);
    let transfer = Transfer::of(&row, transcript.clone());
    let retry = transcript.is_some()
        || (row.sender_id == actions.account.with(|account| account.account_id())
            && matches!(row.status.as_str(), "SENDING" | "FAILED"));
    let name = row
        .media_name
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| presentation::preview(i18n, &row.category, &row.content));
    let size = row
        .media_size
        .filter(|size| *size >= 0)
        .map(|size| file_size(size as u64))
        .unwrap_or_default();
    let markdown = is_markdown(&row);
    let extension = std::path::Path::new(&name)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|_| row.media_name.is_some())
        .map_or_else(|| "FILE".to_owned(), str::to_uppercase);
    let row = stored(row);
    let root = stored(transcript);
    let action = move || {
        if done {
            i18n.tr("Open", "打开")
        } else if transfer.active() {
            i18n.tr("Cancel transfer", "取消传输")
        } else if retry {
            i18n.tr("Retry", "重试")
        } else {
            i18n.tr("Download", "下载")
        }
    };
    ui::command()
        .min_w_0()
        .min_h_0()
        .max_w(relative(1.0))
        .p_0()
        .gap_2()
        .justify_start()
        .accessibility_label(move || Some(action().into()))
        .on_press(move || {
            if !done {
                transfer.toggle(actions);
            } else if markdown {
                actions.open_markdown(row.with(|row| row.clone()), root.with(|root| root.clone()));
            } else {
                let path = row.with(|row| row.media_url.clone().unwrap_or_default());
                actions.task(async move {
                    actions.failed.set(open_path(path).await.is_err());
                });
            }
        })
        .child(
            message_appearance::media_disc()
                .child(when(
                    move || done,
                    move || {
                        text(extension.clone())
                            .number_of_lines(1)
                            .text_role(ui::TextRole::Caption)
                            .text_color(ui::file_extension)
                    },
                ))
                .child(when(
                    move || !done && !transfer.active(),
                    move || {
                        ui::icon(if retry {
                            ui::Icon::Upload
                        } else {
                            ui::Icon::Download
                        })
                    },
                ))
                .child(when(
                    move || !done && transfer.active(),
                    || ui::icon(ui::Icon::Close),
                )),
        )
        .child(
            view()
                .flex_shrink(1.0)
                .min_w_0()
                .gap_1()
                .child(
                    text(name)
                        .text_size(move || appearance.secondary_size())
                        .number_of_lines(1),
                )
                .child(
                    text(move || transfer.percent().unwrap_or_else(|| size.clone()))
                        .text_size(move || appearance.tertiary_size())
                        .text_color(ui::message_secondary),
                ),
        )
        .into_element()
}

fn file_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn contact(row: &MessageListView, actions: Actions) -> AnyElement {
    let appearance = appearance();
    let user_id = row.shared_user_id.clone();
    let name = row.shared_user_full_name.clone().unwrap_or_default();
    let avatar = row.shared_user_avatar_url.clone().unwrap_or_default();
    let avatar_name = name.clone();
    let avatar_id = user_id.clone().unwrap_or_default();
    ui::command()
        .justify_start()
        .gap_2()
        .p_0()
        .min_w_0()
        .min_h_0()
        .on_press(move || {
            if let Some(id) = user_id.clone() {
                actions.show_user(id);
            }
        })
        .child(ui::avatar_view(
            move || avatar_id.clone(),
            move || avatar_name.clone().into(),
            move || avatar.clone(),
            ui::AvatarSize::Medium,
        ))
        .child(
            view()
                .min_w_0()
                .flex_shrink(1.0)
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .min_w_0()
                        .child(
                            text(name)
                                .flex_shrink(1.0)
                                .min_w_0()
                                .number_of_lines(1)
                                .text_size(move || appearance.primary_size()),
                        )
                        .child(identity_badge(
                            false,
                            row.shared_user_is_verified,
                            row.shared_user_app_id.is_some(),
                            row.shared_user_membership.as_deref(),
                        )),
                )
                .child(
                    text(row.shared_user_identity_number.clone().unwrap_or_default())
                        .text_size(move || appearance.secondary_size())
                        .text_color(ui::message_secondary),
                ),
        )
        .into_element()
}

fn transcript_card(
    row: &MessageListView,
    actions: Actions,
    transcript: Option<String>,
    stamp: Option<View>,
) -> AnyElement {
    let i18n = actions.i18n;
    let appearance = appearance();
    let retry = transcript.is_none()
        && row.media_status == "CANCELED"
        && matches!(row.status.as_str(), "SENDING" | "FAILED")
        && row.sender_id == actions.account.with(|account| account.account_id());
    let id = row.message_id.clone();
    let opened = row.clone();
    let mut lines = view()
        .min_w_0()
        .px(ui::space_xs() + ui::space_2xs())
        .py_1()
        .rounded(ui::radius_thumb())
        .bg(ui::quote_background);
    if let Ok(serde_json::Value::Array(previews)) = serde_json::from_str(&row.content) {
        for preview in previews.iter().take(4) {
            lines = lines.child(transcript_line(i18n, preview, appearance));
        }
    }
    let card = ui::command()
        .flex_col()
        .items_stretch()
        .p_0()
        .min_h_0()
        .min_w_0()
        .accessibility_label(move || Some(i18n.text(Key::Transcript).into()))
        .on_press(move || {
            if let Some(root) = transcript.clone() {
                actions.open_transcript_summary(root, opened.message_id.clone());
            } else {
                actions.open_preview(opened.clone());
            }
        })
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .pl_2()
                .pb_1()
                .child(
                    text(move || i18n.text(Key::Transcript))
                        .text_size(move || appearance.primary_size()),
                )
                .child(message_appearance::detail_icon()),
        )
        .child(lines);
    let mut body = view()
        .relative()
        .w(ui::card_width())
        .max_w(relative(1.0))
        .min_w_0()
        .child(card);
    if let Some(stamp) = stamp {
        body = body.child(stamp.absolute().bottom_0().right_0());
    }
    if !retry {
        return body.into_element();
    }
    view()
        .min_w_0()
        .gap_2()
        .child(body)
        .child(ui::action(
            move || i18n.tr("Retry", "重试"),
            move || {
                let client = actions.account.with(|account| account.clone());
                let id = id.clone();
                actions
                    .commit(async move { client.attachment().resume_attachment(id, None).await });
            },
        ))
        .into_element()
}

fn transcript_line(
    i18n: I18n,
    preview: &serde_json::Value,
    appearance: Appearance,
) -> impl IntoElement {
    let field = |key| {
        preview
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
    };
    text(format!(
        "{}: {}",
        field("name"),
        presentation::preview(i18n, field("category"), field("content"))
    ))
    .text_size(move || appearance.tertiary_size())
    .text_color(ui::message_secondary)
    .number_of_lines(1)
}

fn asset_icon(row: &MessageListView, size: ui::AvatarSize) -> nsh_base::AvatarElement {
    let id = row.snapshot_asset_id.clone().unwrap_or_default();
    let symbol = row.snapshot_asset_symbol.clone().unwrap_or_default();
    let icon = row.snapshot_asset_icon_url.clone().unwrap_or_default();
    ui::avatar_view(
        move || id.clone(),
        move || symbol.clone().into(),
        move || icon.clone(),
        size,
    )
}

fn account_snapshot(row: &MessageListView, actions: Actions) -> AnyElement {
    let appearance = appearance();
    let opened = row.clone();
    ui::command()
        .justify_start()
        .gap_2()
        .p_0()
        .min_h_0()
        .min_w_0()
        .disabled(row.snapshot_id.is_none())
        .on_press(move || actions.open_preview(opened.clone()))
        .child(asset_icon(row, ui::AvatarSize::Medium))
        .child(
            view()
                .min_w_0()
                .flex_shrink(1.0)
                .child(
                    text(row.snapshot_amount.clone().unwrap_or_default())
                        .number_of_lines(1)
                        .text_size(move || appearance.secondary_size()),
                )
                .child(
                    text(row.snapshot_asset_symbol.clone().unwrap_or_default())
                        .number_of_lines(1)
                        .text_size(move || appearance.tertiary_size())
                        .text_color(ui::message_secondary),
                ),
        )
        .into_element()
}

fn safe_snapshot(row: &MessageListView, actions: Actions) -> AnyElement {
    let opened = row.clone();
    let amount = row.snapshot_amount.clone().unwrap_or_default();
    // Flutter's steps: the card grows with the amount it holds.
    let width = match amount.chars().count() {
        0..10 => 174.0,
        10..15 => 190.0,
        15..25 => 216.0,
        _ => 232.0,
    };
    let memo = row.snapshot_memo.clone().filter(|memo| !memo.is_empty());
    let mut content = view()
        .min_w_0()
        .p_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(asset_icon(row, ui::AvatarSize::Small).size(ui::icon_sm()))
                .child(
                    text(row.snapshot_asset_symbol.clone().unwrap_or_default())
                        .text_size(ui::asset_text_size()),
                ),
        )
        .child(
            text(amount)
                .mt_4()
                .number_of_lines(1)
                .text_size(ui::amount_text_size())
                .line_height(LineHeight::Multiple(1.0)),
        );
    content = match memo {
        Some(memo) => content.pb(ui::space_2xs()).child(
            text(memo)
                .mt(ui::chip_inset())
                .text_role(ui::TextRole::Caption)
                .text_color(ui::message_secondary)
                .select_text(),
        ),
        None => content.pb(ui::space_xs() + ui::space_2xs()),
    };
    ui::command()
        .relative()
        .flex_col()
        .items_stretch()
        .p_0()
        .min_h_0()
        .min_w_0()
        // Sized from the amount it holds.
        .w(px(width))
        .max_w(relative(1.0))
        .disabled(row.snapshot_id.is_none())
        .on_press(move || actions.open_preview(opened.clone()))
        .child(
            svg(ImageSrc::svg_markup(include_str!(
                "../../assets/message/bg_snapshot.svg"
            )))
            .absolute()
            .top_0()
            .right_0()
            .w(ui::snapshot_backdrop_width())
            .h(ui::snapshot_backdrop_height())
            .accessibility_elements_hidden(true),
        )
        .child(content)
        .into_element()
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
