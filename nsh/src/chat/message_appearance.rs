//! Shared sender, delivery and surface presentation for timeline rows.
use mixin_desktop_api::MessageListView;
use nsh_core::{
    element::{AdaptiveColor, ImageSrc},
    prelude::*,
};

use super::{message_actions::Actions, message_detail, message_system};
use crate::{
    appearance::{self, Appearance, identity_badge},
    presentation,
    ui::{self, Typography},
};

pub(super) fn bubble_surface(mine: bool) -> AdaptiveColor {
    if mine {
        ui::bubble_out()
    } else {
        ui::bubble_in()
    }
}

/// The chat bubble frame, including its fill and shadow for framed content.
pub(super) fn bubble<S: Styled>(
    style: S,
    mine: impl Fn() -> bool + Copy + 'static,
    filled: impl Fn() -> bool + Copy + 'static,
) -> S {
    style
        .relative()
        .flex_shrink(1.0)
        .min_w_0()
        .rounded(ui::radius_md())
        .style_when(filled, move |style| {
            style
                .bg(move || bubble_surface(mine()))
                .shadow_xs()
                .shadow_color(ui::bubble_shadow())
        })
}

/// How far a body stands off its bubble: Flutter's `MessageBubble.padding`.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Inset {
    Regular,
    /// The body reaches the bubble's edges.
    Flush,
    /// Flush, with a caption inset under the picture.
    Caption,
    Transcript,
}

/// Where a message shows its time and delivery.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Stamp {
    /// At the end of the text's last line, or on a line of its own.
    Inline,
    /// Under the bubble, at its end.
    Below,
    /// Drawn by the body over its own content.
    Over,
    Hidden,
}

/// What Flutter's `MessageBubble` is given for each kind of message.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Frame {
    /// Whether the bubble has its fill and nip.
    pub surface: bool,
    /// Takes the incoming fill on either side, as a voice message does.
    pub neutral: bool,
    pub inset: Inset,
    pub stamp: Stamp,
    /// Whether an own message shows its delivery.
    pub delivery: bool,
    /// The body draws its own bubble and nip, so the row leaves it unclipped.
    pub own_bubble: bool,
}

impl Frame {
    pub const TEXT: Self = Self {
        surface: true,
        neutral: false,
        inset: Inset::Regular,
        stamp: Stamp::Inline,
        delivery: true,
        own_bubble: false,
    };

    pub fn of(row: &MessageListView) -> Self {
        let category = row.category.as_str();
        let picture = category.ends_with("_IMAGE");
        let captioned = row
            .caption
            .as_deref()
            .is_some_and(|caption| !caption.trim().is_empty());
        let bare = Self {
            surface: false,
            inset: Inset::Flush,
            stamp: Stamp::Over,
            ..Self::TEXT
        };
        let below = Self {
            stamp: Stamp::Below,
            ..Self::TEXT
        };
        let transfer = Self {
            delivery: false,
            ..below
        };
        let frame = if !message_system::readable(row) {
            Self::TEXT
        } else if picture || category.ends_with("_VIDEO") {
            match captioned {
                false => bare,
                true => Self {
                    inset: Inset::Caption,
                    stamp: if picture { Stamp::Inline } else { Stamp::Over },
                    ..Self::TEXT
                },
            }
        } else if category.ends_with("_STICKER") {
            Self {
                stamp: Stamp::Below,
                ..bare
            }
        } else if category.ends_with("_AUDIO") {
            Self {
                neutral: true,
                ..below
            }
        } else if ["_DATA", "_CONTACT", "_LIVE"]
            .iter()
            .any(|suffix| category.ends_with(suffix))
        {
            below
        } else if category.ends_with("_LOCATION") {
            Self {
                inset: Inset::Flush,
                ..below
            }
        } else if category.ends_with("_POST") {
            Self {
                stamp: Stamp::Over,
                ..Self::TEXT
            }
        } else if category.ends_with("_TRANSCRIPT") {
            Self {
                inset: Inset::Transcript,
                stamp: Stamp::Over,
                ..Self::TEXT
            }
        } else if category == "MESSAGE_RECALL" {
            Self {
                stamp: Stamp::Hidden,
                ..Self::TEXT
            }
        } else if category == "APP_BUTTON_GROUP" {
            Self {
                stamp: Stamp::Hidden,
                ..bare
            }
        } else if category == "APP_CARD" && message_detail::actions_card(&row.content) {
            // The card draws its own surface, and its time at the end of its text.
            Self {
                stamp: Stamp::Over,
                own_bubble: true,
                ..bare
            }
        } else if category == "APP_CARD" {
            below
        } else if category == "SYSTEM_ACCOUNT_SNAPSHOT" {
            transfer
        } else if category == "SYSTEM_SAFE_SNAPSHOT" {
            Self {
                neutral: true,
                ..transfer
            }
        } else if category == "SYSTEM_SAFE_INSCRIPTION" {
            Self {
                neutral: true,
                inset: Inset::Flush,
                ..transfer
            }
        } else {
            Self::TEXT
        };
        // A quotation always sits on a bubble, even over a sticker or a picture.
        let quoted = row
            .quote_message_id
            .as_ref()
            .is_some_and(|id| !id.is_empty());
        Self {
            surface: frame.surface || quoted,
            own_bubble: frame.own_bubble && !quoted,
            ..frame
        }
    }

    /// Insets `body` as the bubble does.
    pub fn padded(self, body: View) -> View {
        match self.inset {
            Inset::Regular => body.p(ui::space_sm()),
            Inset::Flush | Inset::Caption => body,
            Inset::Transcript => body
                .pt(ui::space_xs())
                .pb(ui::space_2xs())
                .px(ui::space_2xs()),
        }
    }
}

/// The disc under a control floating over the timeline. A state fill replaces a
/// button's own, so the surface sits on a parent and hover stacks on it.
pub(super) fn floating_disc() -> View {
    view().rounded(ui::radius_full()).bg(ui::bubble_in)
}

/// The disc of a media state: play, download, or a file's extension.
pub(super) fn media_disc() -> View {
    view()
        .size(ui::status_disc())
        .flex_shrink_0()
        .rounded(ui::radius_full())
        .items_center()
        .justify_center()
        .bg(ui::status_fill)
        .text_color(ui::accent)
}

/// The tail of the last bubble of a run. It lies in the gutter the row leaves at the
/// sender's side, and one point under the bubble so no seam shows between them.
pub(super) fn nip(
    mine: impl Fn() -> bool + Copy + 'static,
    neutral: Memo<bool>,
    highlighted: impl Fn() -> bool + Copy + 'static,
    highlight_opacity: nsh_core::element::AnimatedValue,
) -> impl IntoElement {
    let shape = move |source: &'static str| {
        svg(ImageSrc::svg_markup(source))
            .absolute()
            .bottom(ui::bubble_nip_offset())
            .w(ui::bubble_nip_width() + ui::hairline_width())
            .h(ui::bubble_nip_height())
            .accessibility_elements_hidden(true)
    };
    // The nip is part of the bubble: it casts the bubble's shadow and takes its highlight.
    let layers = move |source: &'static str, side: View| {
        side.child(
            shape(source)
                .bottom(ui::bubble_nip_offset() - ui::hairline_width())
                .text_color(ui::bubble_shadow),
        )
        .child(shape(source).text_color(move || bubble_surface(mine() && !neutral.get())))
        .child(when(highlighted, move || {
            shape(source)
                .text_color(move || {
                    if mine() {
                        ui::bubble_highlight_out()
                    } else {
                        ui::bubble_highlight_in()
                    }
                })
                .bind_opacity(&highlight_opacity)
        }))
    };
    dynamic(move || {
        if mine() {
            layers(
                include_str!("../../assets/message/nip_right.svg"),
                view()
                    .absolute()
                    .right(-ui::bubble_nip_width())
                    .w(ui::bubble_nip_width() + ui::hairline_width())
                    .top(px(0.0))
                    .bottom(px(0.0)),
            )
            .into_element()
        } else {
            layers(
                include_str!("../../assets/message/nip_left.svg"),
                view()
                    .absolute()
                    .left(-ui::bubble_nip_width())
                    .w(ui::bubble_nip_width() + ui::hairline_width())
                    .top(px(0.0))
                    .bottom(px(0.0)),
            )
            .into_element()
        }
    })
}

fn secret(row: &MessageListView) -> bool {
    row.category.starts_with("SIGNAL_") || row.category.starts_with("ENCRYPTED_")
}

/// A message's marks, time and delivery on one line. `delivery` is `None` where the
/// message shows none, and follows status changes without rebuilding the row.
pub(super) fn stamp(row: &MessageListView, delivery: Option<Memo<String>>, on_media: bool) -> View {
    let tone = move || {
        if on_media {
            ui::media_overlay_foreground()
        } else {
            ui::message_status()
        }
    };
    let mark = move |source: &'static str| {
        svg(ImageSrc::svg_markup(source))
            .size(ui::message_status_icon())
            .flex_shrink_0()
            .mr(ui::space_xs())
            .text_color(tone)
    };
    let mut line = view()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .h(ui::message_status_height());
    if row.pinned {
        line = line.child(mark(include_str!("../../assets/message/pin.svg")));
    }
    if secret(row) {
        line = line.child(mark(include_str!("../../assets/message/secret.svg")));
    }
    line = line.child(
        text(presentation::clock(row.created_at_micros / 1000))
            .text_role(ui::TextRole::Micro)
            .text_color(tone),
    );
    if let Some(status) = delivery {
        line = line.child(view().ml(ui::space_sm()).child(dynamic(move || {
            let status = status.get();
            untracked(|| {
                if on_media {
                    appearance::delivery_icon_on_media(&status)
                } else {
                    appearance::delivery_icon(&status)
                }
            })
        })));
    }
    line
}

/// A status over a picture or a transcript card.
pub(super) fn media_stamp(row: &MessageListView, delivery: Option<Memo<String>>) -> View {
    view()
        .px(ui::stamp_inset_x())
        .py(ui::stamp_inset_y())
        .rounded(ui::radius_full())
        .bg(ui::media_stamp)
        .pointer_events(PointerEvents::None)
        .child(stamp(row, delivery, true))
}

/// A status over a post.
pub(super) fn post_stamp(row: &MessageListView, delivery: Option<Memo<String>>) -> View {
    view()
        .px(ui::space_xs() + ui::space_2xs())
        .py(ui::space_2xs())
        .rounded(ui::radius_md())
        .bg(ui::post_stamp)
        .pointer_events(PointerEvents::None)
        .child(stamp(row, delivery, true))
}

/// The room `stamp` needs at the end of the text's last line.
pub(super) fn stamp_reserve(row: &MessageListView, delivery: bool) -> f32 {
    let mark = ui::message_status_icon().0 + ui::space_xs().0;
    let mut width = ui::message_status_spacing().0 + ui::message_status_time().0;
    if row.pinned {
        width += mark;
    }
    if secret(row) {
        width += mark;
    }
    if delivery {
        width += ui::space_sm().0 + ui::icon_xs().0;
    }
    width
}

/// Says that a post or transcript opens larger.
pub(super) fn detail_icon() -> View {
    view()
        .rounded(ui::radius_md())
        .bg(ui::post_stamp)
        .text_color(ui::media_overlay_foreground)
        .pointer_events(PointerEvents::None)
        .child(
            svg(ImageSrc::svg_markup(include_str!(
                "../../assets/message/post_detail.svg"
            )))
            .size(ui::icon_md()),
        )
}

pub(super) fn sender_label(
    name: impl Fn() -> String + 'static,
    id: impl Fn() -> String + 'static,
) -> Text {
    let appearance = use_context::<Appearance>().expect("Home provides chat appearance");
    text(name)
        .text_size(move || appearance.secondary_size())
        .text_color(move || ui::name_color(&id()))
        .number_of_lines(1)
}

pub(super) fn sender(
    message: impl Fn() -> Option<MessageListView> + Copy + 'static,
    actions: Actions,
) -> impl IntoElement {
    let appearance = use_context::<Appearance>().expect("Home provides chat appearance");
    ui::command()
        .justify_start()
        .items_end()
        .min_w_0()
        .min_h_0()
        .px_0()
        .py_0()
        .ml(ui::message_name_inset() - ui::bubble_nip_width())
        .mb(ui::space_2xs())
        .gap(ui::space_2xs())
        .on_press(move || {
            if let Some(row) = message() {
                actions.show_user(row.sender_id);
            }
        })
        .child(sender_label(
            move || message().map(|row| row.sender_name).unwrap_or_default(),
            move || message().map(|row| row.sender_id).unwrap_or_default(),
        ))
        .child(when(
            move || appearance.show_identity(),
            move || {
                text(move || {
                    message()
                        .and_then(|row| row.sender_identity_number)
                        .filter(|id| !id.is_empty() && id != "0")
                        .map(|id| format!("@{id}"))
                        .unwrap_or_default()
                })
                .mb(ui::space_2xs())
                .text_role(ui::TextRole::Micro)
                .text_color(ui::message_identity)
            },
        ))
        .child(view().mb(ui::stamp_inset_y()).child(dynamic(move || {
            let row = message();
            untracked(|| match row {
                Some(row) => identity_badge(
                    false,
                    row.sender_is_verified,
                    row.sender_is_bot,
                    row.sender_membership.as_deref(),
                ),
                None => view().hidden().into_element(),
            })
        })))
        .child(when(
            move || message().is_some_and(|row| row.sender_is_scam),
            move || {
                text(move || actions.i18n.tr("SCAM", "诈骗风险"))
                    .text_role(ui::TextRole::Caption)
                    .text_color(ui::danger)
            },
        ))
}
