//! The glyphs beside menu items: Flutter's `IconFonts` glyphs and its
//! `context_menu_*` images as SVG, drawn in the menu's text color.
use nsh_core::element::ImageSrc;

#[derive(Clone, Copy)]
pub(crate) enum MenuIcon {
    Chat,
    Circle,
    CircleOutline,
    Copy,
    CreateConversation,
    CreateGroup,
    Delete,
    Download,
    Edit,
    Forward,
    ManageCircle,
    ManageUser,
    Mute,
    Pin,
    PositionToChat,
    QrCode,
    Recall,
    Reply,
    SearchUser,
    Select,
    Sticker,
    Stop,
    UnMute,
    UnPin,
}

impl MenuIcon {
    pub(crate) fn src(self) -> ImageSrc {
        ImageSrc::svg_markup(match self {
            Self::Chat => include_str!("../../assets/menu/chat.svg"),
            Self::Circle => include_str!("../../assets/menu/circle.svg"),
            Self::CircleOutline => include_str!("../../assets/menu/circle_outline.svg"),
            Self::Copy => include_str!("../../assets/menu/copy.svg"),
            Self::CreateConversation => include_str!("../../assets/menu/create_conversation.svg"),
            Self::CreateGroup => include_str!("../../assets/menu/create_group.svg"),
            Self::Delete => include_str!("../../assets/menu/delete.svg"),
            Self::Download => include_str!("../../assets/menu/download.svg"),
            Self::Edit => include_str!("../../assets/menu/edit.svg"),
            Self::Forward => include_str!("../../assets/menu/forward.svg"),
            Self::ManageCircle => include_str!("../../assets/menu/manage_circle.svg"),
            Self::ManageUser => include_str!("../../assets/menu/manage_user.svg"),
            Self::Mute => include_str!("../../assets/menu/mute.svg"),
            Self::Pin => include_str!("../../assets/menu/pin.svg"),
            Self::PositionToChat => include_str!("../../assets/menu/position_to_chat.svg"),
            Self::QrCode => include_str!("../../assets/menu/qr_code.svg"),
            Self::Recall => include_str!("../../assets/menu/recall.svg"),
            Self::Reply => include_str!("../../assets/menu/reply.svg"),
            Self::SearchUser => include_str!("../../assets/menu/search_user.svg"),
            Self::Select => include_str!("../../assets/menu/select.svg"),
            Self::Sticker => include_str!("../../assets/menu/sticker.svg"),
            Self::Stop => include_str!("../../assets/menu/stop.svg"),
            Self::UnMute => include_str!("../../assets/menu/un_mute.svg"),
            Self::UnPin => include_str!("../../assets/menu/un_pin.svg"),
        })
    }
}
