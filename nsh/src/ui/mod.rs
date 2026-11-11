//! The app's styled component layer over `nsh_base`. `theme.json` is the design
//! baseline and `nsh/DESIGN.md` its guide: screens compose these components and
//! tokens, and hold no pixels, colors or radii of their own.

mod avatar;
mod button;
mod card;
mod chrome;
mod dialog;
mod feedback;
mod form;
mod icon;
mod markdown;
mod menu_icon;
mod overlay;
mod owned;
pub(crate) mod parts;
pub(crate) mod picker;
mod rows;
mod state;
mod toast;
mod tokens;

pub(crate) use avatar::{
    AvatarSize, avatar_puzzle, avatar_view, circle_color, conversation_avatar, name_color,
};
pub(crate) use button::{
    ButtonSize, ButtonTone, action, button, command, icon_button, link, pill, primary, round_button,
};
pub(crate) use card::{
    card, card_row, checkbox_row, chevron, settings_row, static_row, toggle_row,
};
pub(crate) use chrome::{glass, glass_button, glass_pill, title_bar, title_bar_leading};
pub(crate) use dialog::{
    DialogWidth, dialog, dialog_card, dialog_close, dialog_surface, dialog_title,
    dialog_with_inset, held_dialog, profile_action, profile_actions, profile_bio, profile_card,
    profile_notice, scrim,
};
pub(crate) use feedback::{
    busy_indicator, count_pill, divider, empty, error_text, placeholder, section, section_label,
};
pub(crate) use form::{
    NO_LABEL, checkbox, field, search_input, search_pill, select, select_trigger, slider,
    slider_compact, slider_media, slider_over_scale, slider_over_wave, tab, textarea,
};
pub(crate) use icon::{Icon, icon, inscription_background, logo, wallpaper};
pub(crate) use markdown::{markdown, markdown_reader};
pub(crate) use menu_icon::MenuIcon;
pub(crate) use overlay::{
    context_menu_on, hint, menu, menu_groups, menu_row, menu_row_icon, menu_row_symbol,
    native_context_menu, popover_flush,
};
pub(crate) use owned::owned;
pub(crate) use rows::{list_row, nav_row, option_row};
pub(crate) use state::ring;
pub(crate) use toast::Toasts;
pub(crate) use tokens::*;
