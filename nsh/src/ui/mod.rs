//! The app's styled component layer over `nsh_base`. `theme.json` is the design
//! baseline and `nsh/DESIGN.md` its guide: screens compose these components and
//! tokens, and hold no pixels, colors or radii of their own.

mod avatar;
mod button;
mod card;
mod chrome;
mod feedback;
mod form;
mod icon;
mod overlay;
mod owned;
mod rows;
mod state;
mod tokens;

pub(crate) use avatar::{AvatarSize, avatar_puzzle, avatar_view, conversation_avatar};
pub(crate) use button::{
    ButtonSize, ButtonTone, action, button, command, icon_button, link, primary, round_button,
};
pub(crate) use card::{
    card, card_row, checkbox_row, chevron, settings_row, static_row, toggle_row,
};
pub(crate) use chrome::{title_bar, title_bar_leading};
pub(crate) use feedback::{
    busy_indicator, count_pill, divider, empty, error_text, placeholder, section, section_label,
};
pub(crate) use form::{
    NO_LABEL, checkbox, field, radio, search_input, search_pill, select, select_trigger, slider,
    slider_compact, slider_over_scale, tab, textarea,
};
pub(crate) use icon::{Icon, icon, logo, wallpaper};
pub(crate) use overlay::{
    hint, menu, menu_row, modal, modal_card, modal_title, native_context_menu, popover, scrim,
};
pub(crate) use owned::owned;
pub(crate) use rows::{list_row, nav_row, option_row};
pub(crate) use state::ring;
pub(crate) use tokens::*;
