//! The app's one icon family, and its raster brand assets.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use nsh_core::{
    element::{Image, ImageSrc},
    prelude::*,
};

use super::tokens::icon_md;

#[derive(Clone, Copy)]
pub(crate) enum Icon {
    Chat,
    Search,
    ChevronDown,
    ChevronRight,
    ChevronLeft,
    Rotate,
    ZoomIn,
    ZoomOut,
    Copy,
    Save,
    ArrowLeft,
    Send,
    Globe,
    Lock,
    Check,
    Clock,
    Star,
    Pin,
    Contacts,
    Groups,
    Bot,
    Strangers,
    Circle,
    Collapse,
    Expand,
    Filter,
    Add,
    Sticker,
    Microphone,
    Menu,
    Play,
    Download,
    Upload,
    Close,
    Person,
    Bell,
    Database,
    Shield,
    Brush,
    InfoCircle,
    /// Share: a box with an arrow leaving it.
    Share,
    /// Two speech bubbles: open the conversation.
    Conversation,
    /// A person: open the full profile.
    Profile,
}

pub(crate) fn icon(icon: Icon) -> Svg {
    let asset = |markup: &str| {
        svg(ImageSrc::svg_markup(markup))
            .size(icon_md())
            .flex_shrink_0()
    };
    let stroke = |path: &str| {
        asset(&format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'>{path}</svg>"
        ))
    };
    match icon {
        Icon::Chat => asset(include_str!("../../assets/chat.svg")),
        Icon::Contacts => asset(include_str!("../../assets/slide_contacts.svg")),
        Icon::Groups => asset(include_str!("../../assets/group.svg")),
        Icon::Bot => asset(include_str!("../../assets/bot.svg")),
        Icon::Strangers => asset(include_str!("../../assets/strangers.svg")),
        Icon::Circle => asset(include_str!("../../assets/circle.svg")),
        Icon::Collapse => asset(include_str!("../../assets/collapse.svg")),
        Icon::Expand => asset(include_str!("../../assets/expanded.svg")),
        Icon::Filter => asset(include_str!("../../assets/filter_unseen.svg")),
        Icon::Add => asset(include_str!("../../assets/ic_add.svg")),
        Icon::Search => asset(include_str!("../../assets/ic_search_small.svg")),
        Icon::Sticker => asset(include_str!("../../assets/ic_sticker.svg")),
        Icon::Microphone => asset(include_str!("../../assets/microphone.svg")),
        Icon::Share => asset(include_str!("../../assets/profile_share.svg")),
        Icon::Conversation => asset(include_str!("../../assets/profile_chat.svg")),
        Icon::Profile => asset(include_str!("../../assets/profile_info.svg")),
        Icon::ChevronDown => stroke("<path d='m6 9 6 6 6-6'/>"),
        Icon::ChevronRight => stroke("<path d='m9 6 6 6-6 6'/>"),
        Icon::ChevronLeft => stroke("<path d='m15 6-6 6 6 6'/>"),
        Icon::Rotate => stroke("<path d='M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7'/>"),
        Icon::ZoomIn => {
            stroke("<circle cx='11' cy='11' r='7'/><path d='m20 20-4-4M8 11h6M11 8v6'/>")
        }
        Icon::ZoomOut => stroke("<circle cx='11' cy='11' r='7'/><path d='m20 20-4-4M8 11h6'/>"),
        Icon::Copy => stroke(
            "<rect x='9' y='9' width='11' height='11' rx='2'/><path d='M5 15V6a2 2 0 0 1 2-2h9'/>",
        ),
        Icon::Save => stroke("<path d='M5 4h11l3 3v13H5Z'/><path d='M8 4v5h7V4M8 20v-6h8v6'/>"),
        Icon::ArrowLeft => stroke("<path d='M19 12H5m6-6-6 6 6 6'/>"),
        Icon::Menu => stroke("<path d='M4 6h16M4 12h16M4 18h16'/>"),
        Icon::Play => stroke("<path d='m8 5 11 7-11 7Z'/>"),
        Icon::Download => stroke("<path d='M12 3v12m-5-5 5 5 5-5M5 17v4h14v-4'/>"),
        Icon::Upload => stroke("<path d='M12 16V4m-5 5 5-5 5 5M5 17v4h14v-4'/>"),
        Icon::Close => stroke("<path d='m6 6 12 12M6 18 18 6'/>"),
        Icon::Send => stroke("<path d='m22 2-7 20-4-9-9-4Z'/><path d='M22 2 11 13'/>"),
        Icon::Globe => stroke(
            "<circle cx='12' cy='12' r='9'/><path d='M3 12h18M12 3a17 17 0 0 1 0 18 17 17 0 0 1 0-18Z'/>",
        ),
        Icon::Lock => stroke(
            "<rect x='5' y='10' width='14' height='11' rx='2'/><path d='M8 10V7a4 4 0 0 1 8 0v3'/>",
        ),
        Icon::Clock => stroke("<circle cx='12' cy='12' r='9'/><path d='M12 7v5l3 2'/>"),
        Icon::Star => stroke(
            "<path d='m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.2l-5.6 3 1.1-6.2L3 9.6l6.2-.9Z'/>",
        ),
        Icon::Check => stroke("<path d='m5 12 4 4L19 6'/>"),
        Icon::Pin => stroke("<path d='m16 3 5 5-4 1-4 4-1 5-3-3-6 6 6-6-3-3 5-1 4-4Z'/>"),
        Icon::Person => stroke("<circle cx='12' cy='8' r='4'/><path d='M4 21a8 8 0 0 1 16 0'/>"),
        Icon::Bell => stroke(
            "<path d='M6 9a6 6 0 0 1 12 0c0 6 2.5 8 2.5 8h-17S6 15 6 9Z'/><path d='M10 20.5a2.2 2.2 0 0 0 4 0'/>",
        ),
        Icon::Database => stroke(
            "<ellipse cx='12' cy='5.5' rx='8' ry='3'/><path d='M4 5.5v13c0 1.7 3.6 3 8 3s8-1.3 8-3v-13'/><path d='M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3'/>",
        ),
        Icon::Shield => stroke("<path d='M12 3 5 6v5.5c0 4.5 3 8 7 9.5 4-1.5 7-5 7-9.5V6Z'/>"),
        Icon::Brush => stroke(
            "<path d='M20.5 3.5a2.1 2.1 0 0 0-3 0l-8.8 8.8 3 3 8.8-8.8a2.1 2.1 0 0 0 0-3Z'/><path d='M8.7 12.3C6.5 12.3 5 13.8 5 16c0 1.6-.8 2.6-2 3 1.5 1.3 3.2 2 5 2 3 0 4.7-2.6 3.7-5.7Z'/>",
        ),
        Icon::InfoCircle => stroke("<circle cx='12' cy='12' r='9'/><path d='M12 11v5M12 8h.01'/>"),
    }
}

pub(crate) fn logo() -> Image {
    static SOURCE: std::sync::LazyLock<SharedString> = std::sync::LazyLock::new(|| {
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(include_bytes!("../../assets/logo.png"))
        )
        .into()
    });
    image(SOURCE.clone())
        .size_10()
        .flex_shrink_0()
        .label("Mixin")
}

pub(crate) fn wallpaper() -> Image {
    static SOURCE: std::sync::LazyLock<SharedString> = std::sync::LazyLock::new(|| {
        format!(
            "data:image/png;base64,{}",
            STANDARD.encode(include_bytes!("../../assets/chat_background.png"))
        )
        .into()
    });
    image(SOURCE.clone())
}
