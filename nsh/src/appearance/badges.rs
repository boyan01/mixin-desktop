//! Shared identity and conversation-status assets from the reference client.
use std::sync::LazyLock;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use mixin_desktop_api::active_membership_plan;
use nsh_core::{
    element::{AdaptiveColor, ImageSrc},
    prelude::*,
};

use crate::{
    i18n::{self, Key, tr},
    ui::{self, Typography},
};

pub(crate) fn identity_badge(
    group: bool,
    verified: bool,
    bot: bool,
    membership: Option<&str>,
) -> AnyElement {
    if group {
        return svg(ImageSrc::svg_markup(include_str!(
            "../../assets/badges/group_badge.svg"
        )))
        .size_3p5()
        .text_color(ui::fg_secondary)
        .label(tr("Group", "群组"))
        .into_element();
    }
    if let Some(plan) = active_membership_plan(membership) {
        let (source, label) = match plan {
            "BASIC" => {
                static SOURCE: LazyLock<SharedString> = LazyLock::new(|| {
                    format!(
                        "data:image/png;base64,{}",
                        STANDARD.encode(include_bytes!("../../assets/badges/plan_basic.png"))
                    )
                    .into()
                });
                (SOURCE.clone(), tr("Basic member", "基础会员"))
            }
            "STANDARD" => {
                static SOURCE: LazyLock<SharedString> = LazyLock::new(|| {
                    format!(
                        "data:image/png;base64,{}",
                        STANDARD.encode(include_bytes!("../../assets/badges/plan_standard.png"))
                    )
                    .into()
                });
                (SOURCE.clone(), tr("Standard member", "标准会员"))
            }
            "PREMIUM" => {
                static SOURCE: LazyLock<SharedString> = LazyLock::new(|| {
                    format!(
                        "data:image/gif;base64,{}",
                        STANDARD.encode(include_bytes!("../../assets/badges/plan_premium.gif"))
                    )
                    .into()
                });
                (SOURCE.clone(), tr("Premium member", "高级会员"))
            }
            _ => return view().hidden().into_element(),
        };
        return image(source)
            .size_3p5()
            .object_fit(ResizeMode::Contain)
            .label(label)
            .into_element();
    }
    if verified {
        return svg(ImageSrc::svg_markup(include_str!(
            "../../assets/badges/verified.svg"
        )))
        .size_3p5()
        .label(tr("Verified", "已认证"))
        .into_element();
    }
    if bot {
        return svg(ImageSrc::svg_markup(include_str!(
            "../../assets/badges/bot_fill.svg"
        )))
        .size_3p5()
        .text_color(ui::fg_secondary)
        .label(tr("Bot", "机器人"))
        .into_element();
    }
    view().hidden().into_element()
}

pub(crate) fn status_icon(pinned: bool) -> impl IntoElement {
    let (asset, label) = if pinned {
        (
            include_str!("../../assets/badges/pin.svg"),
            tr("Pinned", "已置顶"),
        )
    } else {
        (
            include_str!("../../assets/badges/mute.svg"),
            tr("Muted", "已静音"),
        )
    };
    svg(ImageSrc::svg_markup(asset))
        .size_3p5()
        .label(label)
        .text_color(ui::fg_secondary)
}

/// Delivery icons preserve the reference geometry; the status remains accessible.
pub(crate) fn delivery_icon(status: &str) -> AnyElement {
    delivery(status, false)
}

pub(crate) fn delivery_icon_on_media(status: &str) -> AnyElement {
    delivery(status, true)
}

/// Over a picture the mark takes the media overlay's foreground.
fn quiet_foreground(on_media: bool) -> AdaptiveColor {
    if on_media {
        ui::media_overlay_foreground()
    } else {
        ui::fg_secondary()
    }
}

fn delivery(status: &str, on_media: bool) -> AnyElement {
    let i18n = i18n::current();
    let (asset, label) = match status {
        "READ" => (
            include_str!("../../assets/badges/read.svg"),
            i18n.text(Key::Read),
        ),
        "DELIVERED" => (
            include_str!("../../assets/badges/delivered.svg"),
            tr("Delivered", "已送达"),
        ),
        "SENT" => (
            include_str!("../../assets/badges/sent.svg"),
            i18n.text(Key::Sent),
        ),
        "FAILED" => {
            return text("!")
                .text_role(ui::TextRole::Body)
                .label(move || i18n.text(Key::Failed))
                .text_color(ui::danger)
                .into_element();
        }
        _ => {
            return text("◷")
                .text_role(ui::TextRole::Body)
                .label(tr("Sending", "发送中"))
                .text_color(move || quiet_foreground(on_media))
                .into_element();
        }
    };
    let read = status == "READ";
    svg(ImageSrc::svg_markup(asset))
        .size_3p5()
        .flex_shrink_0()
        .label(label)
        .text_color(move || {
            if read {
                ui::accent()
            } else {
                quiet_foreground(on_media)
            }
        })
        .into_element()
}
