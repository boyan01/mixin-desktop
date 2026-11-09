//! Typed accessors over `theme.json`, the app's one design baseline.
//!
//! Lengths and numbers are plain values read from the embedded theme file, so a
//! component sets them as static styles: a bound value outranks every static one,
//! which would stop a later `.px_1()` from taking effect. Reading them also
//! subscribes to nothing, and they are right in the first render, before the
//! engine's copy of the theme reaches the core.
//!
//! Colors are functions to pass as bindings: `.bg(ui::surface_card)`.

use nsh_base::{Theme, theme};
use nsh_core::{LineHeight, prelude::*};

thread_local! {
    static BASELINE: Theme = Theme::from_json(include_str!("../../theme.json"))
        .expect("theme.json is part of the build");
}

pub(super) fn number(name: &'static str) -> f32 {
    BASELINE
        .with(|baseline| baseline.number(name))
        .unwrap_or_else(|| panic!("theme.json has no number `{name}`"))
}

fn token_color(name: &'static str) -> AdaptiveColor {
    theme()
        .color(name)
        .or_else(|| BASELINE.with(|baseline| baseline.color(name)))
        .unwrap_or_else(|| panic!("theme.json has no color `{name}`"))
}

// Each scale is complete for the screens to adopt, so a step may have no caller yet.
macro_rules! lengths {
    ($($(#[$doc:meta])* $name:ident = $token:literal;)*) => {$(
        $(#[$doc])*
        #[allow(dead_code)]
        pub(crate) fn $name() -> Points {
            px(number($token))
        }
    )*};
}

macro_rules! colors {
    ($($(#[$doc:meta])* $name:ident = $token:literal;)*) => {$(
        $(#[$doc])*
        #[allow(dead_code)]
        pub(crate) fn $name() -> AdaptiveColor {
            token_color($token)
        }
    )*};
}

lengths! {
    /// An optical correction, such as the gap between two list cells.
    space_2xs = "space-2xs";
    /// Parts of one control.
    space_xs = "space-xs";
    /// Closely related controls.
    space_sm = "space-sm";
    /// One content group.
    space_md = "space-md";
    /// Separate groups in one section.
    space_lg = "space-lg";
    /// Separate sections.
    space_xl = "space-xl";

    /// A framed control: button, field.
    radius_sm = "radius-sm";
    /// A row's fill and a card.
    radius_md = "radius-md";
    /// A floating surface: dialog, popover, menu.
    radius_lg = "radius-lg";
    /// An intentional circle or pill.
    radius_full = "radius-full";

    /// A mark inside compact text, such as a status line.
    icon_xs = "icon-xs";
    icon_sm = "icon-sm";
    icon_md = "icon-md";
    icon_lg = "icon-lg";

    /// A card row.
    row_height = "control-row";
    /// A navigation rail row.
    nav_row_height = "control-nav";
    /// A conversation-sized list cell, margins included.
    list_row_height = "control-list-row";
    /// The height requested for the row shared with the native window controls.
    title_bar_requested_height = "control-title-bar";
    button_height = "control-button";
    button_height_small = "control-button-sm";
    field_height = "control-field";
    /// Six 20pt editor lines plus the content's vertical inset, before text scaling.
    composer_text_max_height = "composer-text-max-height";
    badge_height = "control-badge";
    badge_min_width = "control-badge-min-width";

    focus_ring_width = "focus-ring-width";
    hairline_width = "hairline-width";

    login_width = "login-width";
    login_height = "login-height";
    sidebar_width = "sidebar-width";
    navigation_width = "navigation-width";
    navigation_compact_width = "navigation-compact";
    /// The fixed width of a file or location card: Flutter's map preview.
    card_width = "card-width";
    /// The column of a page. Flutter's `CellGroup` caps a page here.
    content_max_width = "content-max-width";
    /// A decision, a short form or a person: Flutter's `UserDialog`.
    dialog_width_compact = "dialog-width-compact";
    /// A dialog with a list or a longer form.
    dialog_width_regular = "dialog-width-regular";
    /// A dialog with room for a document.
    dialog_width_wide = "dialog-width-wide";
    /// A dialog that holds a page or a long list.
    dialog_height_tall = "dialog-height-tall";
    picker_width = "picker-width";
    picker_height = "picker-height";
    picker_form_height = "picker-form-height";
    viewer_toolbar_height = "viewer-toolbar-height";
    viewer_toolbar_wide = "viewer-toolbar-wide";
    dialog_close_size = "dialog-close-size";
    /// One command of a `profile_actions` row, and the icon in it.
    profile_action_size = "profile-action-size";
    profile_action_icon = "profile-action-icon";
    /// How far a `profile_actions` row stands off the edges of its card.
    profile_action_inset = "profile-action-inset";
    /// Where a `profile_bio` starts to scroll.
    profile_bio_height = "profile-bio-height";
    sticker_panel_width = "sticker-panel-width";
    sticker_panel_height = "sticker-panel-height";
    sticker_cell_size = "sticker-cell-size";
    sticker_art_size = "sticker-art-size";
    /// The sticker panel's bottom tab bar, and one tab in it.
    sticker_tab_bar_height = "sticker-tab-bar-height";
    sticker_tab_size = "sticker-tab-size";
    radius_login = "radius-login";
    radius_composer = "radius-composer";
    /// A day separator and a notice in the timeline.
    radius_chip = "radius-chip";
    /// A thumbnail or a block inside a bubble.
    radius_thumb = "radius-thumb";
    /// A bot's icon in its card.
    radius_app_icon = "radius-app-icon";

    /// Flutter's bubble nip: its box, and how far it stands off the bubble's bottom.
    bubble_nip_width = "bubble-nip-width";
    bubble_nip_height = "bubble-nip-height";
    bubble_nip_offset = "bubble-nip-offset";
    /// The floating pinned-message card: its height, side inset, nip and buttons.
    pin_banner_height = "pin-banner-height";
    /// One thumbnail in the send dialog's attachment grid.
    thumbnail_grid_size = "thumbnail-grid-size";
    pin_banner_inset = "pin-banner-inset";
    pin_nip_width = "pin-nip-width";
    pin_nip_height = "pin-nip-height";
    pin_button_size = "pin-button-size";
    pin_close_size = "pin-close-size";
    /// What a bubble leaves free on the side away from its sender.
    message_far_edge = "message-far-edge";
    /// Where a sender's name starts, from the bubble's nip gutter.
    message_name_inset = "message-name-inset";
    /// The line of a message's time and delivery, and the marks in it.
    message_status_height = "message-status-height";
    message_status_icon = "message-status-icon";
    /// The gap between a message's text and the status at the end of its last line.
    message_status_spacing = "message-status-spacing";
    /// The room a message's `HH:mm` takes at the status size.
    message_status_time = "message-status-time";
    /// The side inset of a chip, and the vertical inset of a caption.
    chip_inset = "chip-inset";
    notice_inset_y = "notice-inset-y";
    day_chip_min_width = "day-chip-min-width";
    /// The widest one-line notice, such as a pinned message.
    notice_max_width = "notice-max-width";
    /// A message's status over a picture.
    stamp_inset_x = "stamp-inset-x";
    stamp_inset_y = "stamp-inset-y";
    /// The disc of a media state: play, download, a file's extension.
    status_disc = "status-disc";
    /// The check column that opens beside messages while selecting.
    selection_column_width = "selection-column";
    audio_wave_width = "audio-wave-width";
    audio_wave_height = "audio-wave-height";
    /// The play and stop marks of a voice message.
    audio_icon = "audio-icon";
    /// The bounds of a sticker's sides in a message.
    sticker_min_size = "sticker-min-size";
    sticker_max_size = "sticker-max-size";
    /// The artwork at the corner of a transfer card.
    snapshot_backdrop_width = "snapshot-backdrop-width";
    snapshot_backdrop_height = "snapshot-backdrop-height";
    post_max_width = "post-max-width";
    /// The reading column, including its side insets.
    post_reader_width = "post-reader-width";
    quote_min_height = "quote-min-height";
    quote_bar_width = "quote-bar-width";
    /// The bounds of a bot's button group, and the side inset of a button in it.
    action_group_min_width = "action-group-min-width";
    action_group_max_width = "action-group-max-width";
    action_button_inset_x = "action-button-inset-x";
    actions_card_min_width = "actions-card-min-width";
    actions_card_max_width = "actions-card-max-width";
    stranger_button_min_width = "stranger-button-min-width";
    inscription_height = "inscription-height";
    /// The amount and the asset of a transfer card.
    amount_text_size = "text-amount-size";
    asset_text_size = "text-asset-size";
}

colors! {
    /// The navigation rail.
    surface_rail = "sidebar";
    /// The conversation or settings list column, and the bars over a pane.
    surface_list = "list-background";
    /// A detail pane.
    surface_pane = "background";
    /// The message timeline and the empty state beside the list.
    surface_chat = "chat-background";
    /// Grouped rows: one tone above whichever surface it sits on.
    surface_card = "card";
    /// Menus, popovers, dialogs and tooltips.
    surface_popover = "popover";
    /// The sign-in screen behind its card.
    surface_landing = "landing-background";

    /// A message from someone else, and the controls floating over the timeline.
    bubble_in = "bubble-in";
    /// A message from the account.
    bubble_out = "bubble-out";
    /// The chip of a day separator.
    day_background = "day-background";
    day_foreground = "day-foreground";
    /// The chip of a system or pin notice.
    notice_background = "notice-background";
    notice_foreground = "notice-foreground";
    /// The chip of the end-to-end encryption notice.
    encrypt_background = "encrypt-background";
    /// A message's time and its marks.
    message_status = "message-status";
    /// Quiet text in a bubble, and delivery marks: Flutter's `secondaryText`.
    message_secondary = "message-secondary";
    /// A sender's identity number beside the name.
    message_identity = "message-identity";
    /// A reply's quotation inside a bubble.
    quote_background = "quote-background";
    /// The elevation under a bubble.
    bubble_shadow = "bubble-shadow";
    /// What marks a located message over its bubble.
    bubble_highlight_in = "bubble-highlight-in";
    bubble_highlight_out = "bubble-highlight-out";
    bubble_highlight_media = "bubble-highlight-media";
    /// A voice message's bars: to play, and played.
    waveform_track = "waveform-track";
    waveform_progress = "waveform-progress";
    /// The disc of a media state, and the light fill of a `pill`.
    status_fill = "status-background";
    /// The extension in a file's disc.
    file_extension = "file-extension";
    /// A message's status over a picture, and over a post.
    media_stamp = "media-stamp";
    post_stamp = "post-stamp";
    /// A chip over a picture, in either appearance.
    media_overlay = "media-overlay";
    media_overlay_foreground = "media-overlay-foreground";

    /// Text and icons that carry the content.
    fg_primary = "foreground";
    /// Metadata, previews, placeholders and disclosure arrows.
    fg_secondary = "muted-foreground";
    /// What may go unread: inactive decoration.
    fg_tertiary = "foreground-tertiary";
    /// What is drawn on an `accent` fill.
    fg_on_accent = "primary-foreground";

    /// The fill over any surface while a pointer is over a row or quiet control.
    state_hover = "state-hover";
    /// The fill while pressed.
    state_pressed = "state-pressed";
    /// The persistent fill of a selected row. It stacks on `surface_card`.
    state_selected = "state-selected";
    focus_ring = "ring";

    /// The principal action and selection emphasis.
    accent = "primary";
    /// Destructive actions, failures and unread alerts.
    danger = "destructive";
    success = "success";

    /// A boundary inside a pane.
    hairline = "hairline";
    /// A boundary between panes and around floating surfaces.
    border = "border";
    /// The fill of a field and of the search pill.
    input_fill = "input-background";
    picker_search_fill = "picker-search-background";
    /// The frame of a field at rest.
    input_border = "input";
    /// The dimming layer behind modal surfaces, dark in either appearance.
    scrim_fill = "scrim";
    viewer_background = "viewer-background";
    viewer_toolbar_fill = "viewer-toolbar-background";
    viewer_secondary = "viewer-secondary";
    /// A count pill that does not ask for attention.
    badge_quiet = "badge-quiet";
    /// The placeholder of an avatar with no id to pick a color from.
    avatar_empty = "avatar-empty";
}

/// The opacity of a control that does not take input.
pub(super) fn opacity_disabled() -> f32 {
    number("opacity-disabled")
}

/// The opacity of an accent fill under a pointer.
pub(super) fn opacity_solid_hover() -> f32 {
    number("opacity-solid-hover")
}

/// The opacity of an accent fill while pressed.
pub(super) fn opacity_solid_pressed() -> f32 {
    number("opacity-solid-pressed")
}

/// The opacity of the chat wallpaper over its surface.
pub(crate) fn chat_pattern_opacity() -> f32 {
    number("chat-pattern-opacity")
}

/// The app's type scale. Add a size only by adding a role to `theme.json`.
#[derive(Clone, Copy)]
pub(crate) enum TextRole {
    /// Delivery metadata, which stays at one size when message text scales.
    Micro,
    /// Timestamps, counts and helper text.
    Caption,
    /// Default text: previews, descriptions, controls.
    Body,
    /// Row titles: conversation names and card rows.
    Label,
    /// Bar and page titles.
    Title,
    /// Dialog titles and profile names.
    Headline,
}

impl TextRole {
    fn tokens(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Micro => ("text-micro-size", "text-micro-line", "text-micro-weight"),
            Self::Caption => (
                "text-caption-size",
                "text-caption-line",
                "text-caption-weight",
            ),
            Self::Body => ("text-body-size", "text-body-line", "text-body-weight"),
            Self::Label => ("text-label-size", "text-label-line", "text-label-weight"),
            Self::Title => ("text-title-size", "text-title-line", "text-title-weight"),
            Self::Headline => (
                "text-headline-size",
                "text-headline-line",
                "text-headline-weight",
            ),
        }
    }
}

pub(crate) trait Typography: Styled {
    /// Sets the size, line height and weight of `role`. Text scales with the
    /// user's text size, so lines are a multiple of it.
    fn text_role(self, role: TextRole) -> Self {
        let (size, line, weight) = role.tokens();
        self.text_size(px(number(size)))
            .line_height(LineHeight::Multiple(number(line)))
            .font_weight(FontWeight(number(weight) as u16))
    }

    /// The heavier weight of a control's label or a section's heading.
    fn text_emphasis(self) -> Self {
        self.font_weight(FontWeight(number("text-emphasis-weight") as u16))
    }
}

impl<T: Styled> Typography for T {}
