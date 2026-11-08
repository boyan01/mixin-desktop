# Design baseline

`nsh/theme.json` is the one design baseline and `nsh/src/ui/` the one styled layer
over `nsh_base`. Values follow the Flutter app; the dark theme tracks it closely. Where the desktop reads better differently, the
desktop wins: transient feedback is a bottom-centered `ui::Toasts` card (2s success,
5s error), and the send dialog is a thumbnail grid (`thumbnail-grid-size`) instead of a list.

**The rule: screens use `ui::` components and tokens only. No raw `px`, colors or
radii in screen modules.** A missing role or variant goes into `theme.json` or
`ui/` first. Documented exceptions are physical or measured geometry, such as a
bitmap size. `nsh/tool/check-ui-literals.sh` reports what remains per file.

## Tokens

Accessors live in `ui/tokens.rs`. Lengths are plain values, read from the embedded
theme: use them as static styles (`.px(ui::space_md())`), because a bound value
outranks every static one and stops a later override. Colors are functions, passed
as bindings (`.bg(ui::surface_card)`). Tailwind steps with the same value
(`.gap_2()`, `.p_4()`) are equally on the scale.

| Group | Tokens | Values |
| --- | --- | --- |
| Space | `space_2xs` `xs` `sm` `md` `lg` `xl` | 2 (optical only) 4 8 12 16 24 |
| Radius | `radius_sm` `md` `lg` `full` | 5 framed control, 8 fill and card, 11 floating, pill |
| Radius | `radius_app_icon` `radius_thumb` `radius_chip` | 4 bot icon, 6 thumbnail, 10 timeline chip |
| Icon | `icon_xs` `sm` `md` `lg` | 14 16 20 24 |
| Avatar | `AvatarSize::Small` `Compact` `Bar` `Medium` `Large` `Hero` | 24 32 36 40 50 120 |
| Height | `row_height` `nav_row_height` `list_row_height` | 52 40 80 |
| Height | `title_bar_requested_height` `button_height` `button_height_small` `field_height` | 52 36 32 40 |
| Layout | `sidebar_width` `navigation_width` `navigation_compact_width` `card_width` | 300 176 64 260 |
| Layout | `content_max_width` `dialog_height_tall` | 600 640 |
| Layout | `DialogWidth::Compact` `Regular` `Wide` | 340 384 640 |
| Profile | `dialog_close_size` `profile_action_size` `profile_action_icon` `profile_action_inset` `profile_bio_height` | 40 46 30 45 120 |
| Layout | `sticker_panel_width` `sticker_panel_height` | 464 407 |
| Bubble | `bubble_nip_width` `bubble_nip_height` `bubble_nip_offset` `message_far_edge` `message_name_inset` | 9 12 9 65 10 |
| Status | `message_status_height` `message_status_icon` `message_status_spacing` `message_status_time` `stamp_inset_x` `stamp_inset_y` | 12 8 6 28 5 3 |
| Notice | `chip_inset` `notice_inset_y` `day_chip_min_width` `notice_max_width` | 10 5 64 400 |
| Message | `status_disc` `audio_wave_width` `audio_wave_height` `audio_icon` `post_max_width` `quote_min_height` `quote_bar_width` | 38 238 12 12 400 50 6 |
| Message | `sticker_min_size` `sticker_max_size` `inscription_height` `snapshot_backdrop_width` `snapshot_backdrop_height` | 60 140 112 86 67 |
| Bot | `action_group_min_width` `action_group_max_width` `action_button_inset_x` `actions_card_min_width` `actions_card_max_width` `stranger_button_min_width` | 240 340 14 320 375 162 |

| `TextRole` | Size / weight | Use |
| --- | --- | --- |
| `Micro` | 10 / 400 | Delivery metadata, fixed when message text scales |
| `Caption` | 12 / 400 | Timestamps, counts, helper text |
| `Body` | 14 / 400 | Default text, previews, controls |
| `Label` | 16 / 400 | Row titles: conversation names, card rows |
| `Title` | 16 / 600 | Bar and page titles |
| `Headline` | 18 / 600 | Dialog titles, profile names |

Apply with `.text_role(ui::TextRole::Body)` from `ui::Typography`. `.text_emphasis()`
is the 500 weight of a control label or section heading.

## Surfaces and foreground

Surfaces are ordered by stacking, not brightness. As in Flutter, a dark detail pane
is darker than the list column beside it.

| Level | Token (theme name) | Light | Dark |
| --- | --- | --- | --- |
| Rail | `surface_rail` (`sidebar`) | `#f7f7f7` | `#2e3338` |
| List and bars | `surface_list` (`list-background`) | `#ffffff` | `#2c3136` |
| Pane | `surface_pane` (`background`) | `#f6f7fa` | `#282c30` |
| Chat and empty | `surface_chat` (`chat-background`) | `#edeeee` | `#23272b` |
| Card | `surface_card` (`card`), translucent | black 4% | white 6% |
| Popover | `surface_popover` (`popover`) | `#ffffff` | `#3e4148` |
| Landing | `surface_landing` (`landing-background`) | `#e5e5e5` | `#23272b` |

| Foreground | Use | Light | Dark |
| --- | --- | --- | --- |
| `fg_primary` | Content text and icons | `#333333` | `#e9eaeb` |
| `fg_secondary` | Metadata, previews, placeholders, chevrons | `#9298a3` | `#85898e` |
| `fg_tertiary` | Inactive decoration | `#b8bdc7` | `#6b6f73` |
| `fg_on_accent` | On an `accent` fill | `#ffffff` | `#ffffff` |

Status and structure: `accent`, `danger`, `success`, `hairline` (inside a pane),
`border` (between panes, around floating surfaces), `input_fill`, `scrim_fill`,
`badge_quiet`, and `circle_color(id)` and `name_color(id)`, which take Flutter's
palettes and id hash from `ui/avatar.rs`.

The timeline has its own roles, all with Flutter's values, and floating controls
share `bubble_in`. A bubble has no fixed width: it ends `message_far_edge` from the
pane's far side. `message_appearance::Frame` says how each message kind is framed.

| Role | Token | Light | Dark |
| --- | --- | --- | --- |
| Incoming bubble | `bubble_in` | `#ffffff` | `#343b43` |
| Own bubble | `bubble_out` | `#c5edfd` | `#3b4f67` |
| Bubble elevation | `bubble_shadow` | black 25% | black 25% |
| Day chip and text | `day_background` `day_foreground` | `#d5d3f3` black | `#d5d3f3` black |
| Notice chip and text | `notice_background` `notice_foreground` | `#caeac9` black | `#caeac9` black |
| Encryption chip | `encrypt_background` | `#fff7ad` | `#fff7ad` |
| Time and marks | `message_status` | `#83919e` | `#808386` |
| Quiet bubble text, delivery | `message_secondary` | `#b8bdc7` | white 40% |
| Identity number | `message_identity` | `#333333` 50% | white 45% |
| Quote and transcript block | `quote_background` | black 4% | black 4% |
| Waveform | `waveform_track` `waveform_progress` | `#dddddd` `#9b9b9b` | white 40%, white |
| Media state disc | `status_fill` | `#f5f7fa` | `#f5f7fa` |
| File extension | `file_extension` | `#b8bdc7` | `#b8bdc7` |
| Status over a picture, a post | `media_stamp` `post_stamp` | black 30%, black 20% | black 30%, black 20% |
| Located message | `bubble_highlight_in` `out` `media` | black 13%, 16%, 20% | white 12%, black 18%, 20% |
| Chip over a picture | `media_overlay` | black 60% | black 60% |
| Text over a picture | `media_overlay_foreground` | `#ffffff` | `#ffffff` |

## States

`ui/state.rs` owns the one treatment. Overlays are translucent, so they read on
every surface, and a selected row stacks on its card.

| State | Token | Light | Dark |
| --- | --- | --- | --- |
| Hover | `state_hover` | black 4% | white 3% |
| Selected | `state_selected` | black 8% | white 6% |
| Pressed | `state_pressed` | black 12% | white 10% |
| Disabled | `opacity-disabled` | 0.4 | 0.4 |
| Focus | `focus_ring`, `focus-ring-width` | 2pt outline | 2pt outline |

An accent fill uses `opacity-solid-hover` and `opacity-solid-pressed` instead.
Inside a clipping `card`, the ring is drawn inward. A focusable surface that is not
a component takes `.focus(ui::ring)`.

## Components

| Component | Use |
| --- | --- |
| `title_bar` | Any strip level with the native controls: it drags the window, double click follows the system. Pad with `title_bar_leading(offset)` |
| `nav_row` | A rail row, or a compact pressable identity |
| `list_row` | A conversation-sized list cell, 80pt with margins |
| `card`, `card_row` | A group of rows, and a free-form pressable row in it |
| `settings_row`, `chevron` | An icon and label row; add `chevron` when it leads somewhere |
| `static_row`, `toggle_row` | A label with a value or control; a label with a switch |
| `checkbox_row` | One independent choice in a `card`; add its content after the mark |
| `button(tone, size)` | Tones `Primary`, `Secondary`, `Ghost`, `Danger`; sizes `Regular`, `Small` |
| `primary`, `action`, `command` | Shorthand for a regular primary, labeled secondary and ghost button |
| `pill` | A light, accent-labeled command that invites rather than commits, such as adding a contact |
| `link` | A command read inside running text, in the accent color |
| `icon_button`, `round_button` | An icon-only command in content; in a bar. Always give an accessible name |
| `field`, `textarea` | Text input |
| `search_pill`, `search_input` | The search frame and its frameless input |
| `select`, `select_trigger`, `switch` | Choice controls. Use `switch` through `toggle_row` in settings |
| `checkbox`, `radio` | An independent choice; one of a `radio_group`. `NO_LABEL` shows none: give an accessible name or add content |
| `slider`, `slider_compact`, `slider_over_scale`, `slider_over_wave` | A value on a scale; inside a message or strip; over a scale the caller draws; over a waveform |
| `tab` | One tab of a base `tabs` row, sharing its width |
| `option_row` | An option of a listbox driven from an input; never takes focus |
| `count_pill` | A count; emphasized only when it asks for attention |
| `avatar_view`, `conversation_avatar` | A person, whose id picks the placeholder color; a conversation with the group puzzle. Both take an `AvatarSize` |
| `avatar_puzzle` | The one-to-four tile puzzle, inside a clipping frame |
| `section`, `section_label` | A headed group of cards; the heading alone |
| `divider` | A hairline between groups of one pane |
| `menu`, `menu_row`, `popover` | Anchored surfaces. They and `select` bring the popup treatment; add none at the call site |
| `native_context_menu`, `menu_groups` | The OS menu on a secondary click; its item groups joined by separators |
| `hint` | The tooltip of a control whose label is hidden |
| `dialog`, `held_dialog`, `dialog_card(width)`, `dialog_title`, `dialog_close`, `scrim` | A blocking decision: scrim, motion and dismissal. A `held_dialog` ignores a stray press on the scrim; a `dialog_surface` styles a card that changes shape |
| `profile_card`, `profile_bio`, `profile_notice`, `profile_actions`, `profile_action` | A person in a dialog: picture, name and muted line, then a `pill`, biography or notice, and a row of icon commands |
| `error_text`, `placeholder`, `empty`, `busy_indicator` | Failure, empty pane, empty timeline or list, real pending work |

`Primary` marks the one default commitment of a decision area, not the only button.

The chat composer starts as one compact row. Attachment and sticker commands sit
on the left; one fixed trailing slot crossfades between microphone (empty draft)
and Send (text, attachments or a pending send). The editor grows upward to about
six visible lines before scrolling, with bottom-aligned tools and a short native
layout transition. Recording replaces the whole row with cancel, waveform and
duration; the trailing Stop action becomes Send after capture, with inline playback.
Text/voice and capture/preview surfaces crossfade without discarding the editor
document. Motion honors reduced-motion settings. Recording and playback reuse the
same 16pt composer radius and compact row geometry.
Document attachments fit their filename/size up to the card-width cap; pictures retain
square thumbnails. Tooltips and accessible names explain hidden toolbar labels.
The editor keeps a neutral border while focused; its caret indicates text focus,
while individual keyboard controls retain their focus rings. Original-image and
ZIP choices are conditional icon toggles beside the attachment previews. Original
images light the image control; ZIP appears for multiple attachments. ZIP keeps
original files, temporarily locks image quality and folds the previews into one
counted archive summary with a short native height/opacity animation. Both preview
layers retain their own geometry, and packaging preserves the source thumbnails.
Unfolding restores the previous image quality. The archive summary's lock opens
the optional ZIP password. While attachments are present, the input row hides
attachment and emoji commands; the preview's plus adds more files, and the trailing
action remains Send even with an empty caption.
Reprocessing stays inside the attachment area without inserting a status row.
Voice previews keep the waveform and elapsed/total time together, with waveform
width capped at `audio-wave-width`; the trailing action remains at the right edge.
Played bars, pause/resume and seeking follow the account's shared playback client,
which also owns the recorder within that account scope.
