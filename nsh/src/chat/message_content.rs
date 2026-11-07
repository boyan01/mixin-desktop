use super::{
    message_actions::Actions, message_detail, message_media::Transfer, message_system, message_text,
};
use crate::{
    appearance::Appearance,
    i18n::{I18n, Key},
    presentation,
    ui::{self, Typography},
};
use mixin_desktop_api::MessageListView;
use nsh_base::text_view;
use nsh_core::{element::ImageSrc, prelude::*};

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
}

pub(super) fn body(row: MessageListView, actions: Actions, placement: Placement) -> AnyElement {
    if let Some(unavailable) = message_system::unavailable(&row, actions) {
        return unavailable;
    }
    if message_system::centered(&row.category) {
        return message_system::notice(&row, actions, placement.target);
    }
    let appearance = appearance();
    let transcript = placement.transcript.clone();
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
            placement.target,
            transcript.is_some(),
        ));
    }
    content = content.child(kind_body(&row, actions, placement));
    if let Some(caption) = row.caption.as_deref().filter(|s| !s.is_empty()) {
        content = content.child(message_text::read(&row, caption, actions, transcript));
    }
    content.into_element()
}

fn kind_body(row: &MessageListView, actions: Actions, placement: Placement) -> AnyElement {
    let category = row.category.as_str();
    let Placement {
        transcript, status, ..
    } = placement;
    if category.ends_with("_TEXT") {
        message_text::read(row, &row.content, actions, transcript).into_element()
    } else if category == "MESSAGE_RECALL" {
        recalled(row, actions)
    } else if category.ends_with("_POST") {
        post(row, actions)
    } else if category.ends_with("_STICKER") {
        sticker(row, actions)
    } else if category.ends_with("_IMAGE") || category.ends_with("_VIDEO") {
        super::message_media::card(row.clone(), transcript, actions, status).into_element()
    } else if category.ends_with("_AUDIO") && available(row) {
        super::audio::body(row.clone(), transcript, actions).into_element()
    } else if is_attachment(row) {
        file(row.clone(), transcript, actions)
    } else if category.ends_with("_CONTACT") {
        contact(row, actions)
    } else if category.ends_with("_TRANSCRIPT") {
        transcript_card(row, actions, transcript)
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
    } else if category == "SYSTEM_ACCOUNT_SNAPSHOT" || category == "SYSTEM_SAFE_SNAPSHOT" {
        snapshot(row, actions)
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
        .gap_2()
        .child(
            text(move || {
                if mine {
                    i18n.tr("You deleted this message", "你删除了这条消息")
                } else {
                    i18n.tr("This message was deleted", "这条消息已被删除")
                }
            })
            .text_color(ui::fg_secondary),
        )
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

fn post(row: &MessageListView, actions: Actions) -> AnyElement {
    let i18n = actions.i18n;
    let appearance = appearance();
    let preview = row.clone();
    // Flutter's post width, also the height bound. The label takes one `space_xl` line.
    let bound = px(400.0);
    ui::command()
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
                .max_h(bound - ui::space_xl())
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
                .text_role(ui::TextRole::Caption)
                .text_color(ui::fg_secondary),
        )
        .into_element()
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
    ui::command()
        .p_0()
        .min_h_0()
        .disabled(id.is_none())
        .accessibility_label(move || Some(i18n.text(Key::Sticker).into()))
        .on_press(move || actions.open_sticker(pressed.clone()))
        .child(crate::sticker::asset(
            actions.account.with(|value| value.clone()),
            move || (id.clone(), url.clone(), kind.clone()),
            160.0,
        ))
        .into_element()
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
        .max_w(relative(1.0))
        .w(ui::card_width())
        .p_1()
        .gap_3()
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
            view()
                .size_10()
                .flex_shrink_0()
                .rounded(ui::radius_full())
                .items_center()
                .justify_center()
                .bg(ui::surface_card)
                .text_color(ui::fg_secondary)
                .child(when(
                    move || done,
                    || {
                        svg(ImageSrc::svg_markup(include_str!(
                            "../../assets/quote/file.svg"
                        )))
                        .size(ui::icon_md())
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
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    text(name)
                        .text_size(move || appearance.secondary_size())
                        .number_of_lines(2),
                )
                .child(
                    text(move || transfer.percent().unwrap_or_else(|| size.clone()))
                        .text_size(move || appearance.tertiary_size())
                        .text_color(ui::fg_secondary),
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
    ui::command()
        .justify_start()
        .gap_3()
        .p_1()
        .min_w_0()
        .on_press(move || {
            if let Some(id) = user_id.clone() {
                actions.show_user(id);
            }
        })
        .child(ui::avatar_view(
            move || avatar_name.clone().into(),
            move || avatar.clone(),
            ui::AvatarSize::Medium,
        ))
        .child(
            view()
                .min_w_0()
                .gap_1()
                .child(text(name).number_of_lines(1).text_emphasis())
                .child(
                    text(
                        row.shared_user_identity_number
                            .clone()
                            .map(|id| format!("Mixin ID: {id}"))
                            .unwrap_or_default(),
                    )
                    .text_size(move || appearance.tertiary_size())
                    .text_color(ui::fg_secondary),
                ),
        )
        .into_element()
}

fn transcript_card(
    row: &MessageListView,
    actions: Actions,
    transcript: Option<String>,
) -> AnyElement {
    let i18n = actions.i18n;
    let appearance = appearance();
    let retry = transcript.is_none()
        && row.media_status == "CANCELED"
        && matches!(row.status.as_str(), "SENDING" | "FAILED")
        && row.sender_id == actions.account.with(|account| account.account_id());
    let id = row.message_id.clone();
    let opened = row.clone();
    let mut card = ui::command()
        .flex_col()
        .items_start()
        .gap_1()
        .p_1()
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
            text(move || i18n.text(Key::Transcript))
                .text_size(move || appearance.secondary_size())
                .text_emphasis(),
        );
    if let Ok(serde_json::Value::Array(previews)) = serde_json::from_str(&row.content) {
        for preview in previews.iter().take(4) {
            card = card.child(transcript_line(i18n, preview, appearance));
        }
    }
    let card = view().gap_2().min_w_0().child(card);
    if !retry {
        return card.into_element();
    }
    card.child(ui::action(
        move || i18n.tr("Retry", "重试"),
        move || {
            let client = actions.account.with(|account| account.clone());
            let id = id.clone();
            actions.commit(async move { client.attachment().resume_attachment(id, None).await });
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
    .text_color(ui::fg_secondary)
    .number_of_lines(1)
}

fn snapshot(row: &MessageListView, actions: Actions) -> AnyElement {
    let appearance = appearance();
    let opened = row.clone();
    let mut card = view().gap_2().min_w_0().child(
        ui::command()
            .flex_col()
            .items_start()
            .disabled(row.snapshot_id.is_none())
            .on_press(move || actions.open_preview(opened.clone()))
            .child(
                text(format!(
                    "{} {}",
                    row.snapshot_amount.clone().unwrap_or_default(),
                    row.snapshot_asset_symbol.clone().unwrap_or_default()
                ))
                .text_emphasis(),
            ),
    );
    if let Some(memo) = row.snapshot_memo.clone().filter(|s| !s.is_empty()) {
        card = card.child(
            text(memo)
                .text_size(move || appearance.primary_size())
                .select_text(),
        );
    }
    card.into_element()
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
