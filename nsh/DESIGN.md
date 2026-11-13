# Design baseline

`nsh/theme.json` is the one design baseline and `nsh/src/ui/` the one styled layer
over `nsh_base`. Values follow the Flutter app; the dark theme tracks it closely. Where the desktop reads better differently, the
desktop wins: transient feedback is a bottom-centered `ui::Toasts` card (2s success,
5s error), and the send dialog is a thumbnail grid (`thumbnail-grid-size`) instead of a list.

The chat's chrome floats over the timeline as glass capsules: the title bar's
identity and each of its buttons, the latest pin (in the title row from
`glass-inline-pin-width`, on a row of its own below that), the jump buttons, the
announcement, the composer and toasts. `ui::glass` draws Liquid Glass where the
platform has it (macOS 26 and later) and a `glass-blur` backdrop under a
`glass-tint-opacity` tint elsewhere. Content, such as bubbles, cards and settings
lists, stays solid.

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
| Height | `row_height` `nav_row_height` `list_row_height` `group_row_height` | 52 40 80 44 |
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

Snapshot details use the regular 384pt dialog surface in both chat and link
navigation. One fixed header owns the title and close icon; the body owns vertical
scrolling, with 24pt content insets. Transfer details start with an asset/chain
icon, a 32pt amount and a body-sized symbol on the same baseline. Incoming and
outgoing amounts retain their sign and precision; pending amounts stay neutral.
Available current and historical estimates are explicitly denominated in USD.
Transaction fields follow the snapshot type, with full, selectable identifiers,
quiet copy buttons and a local timestamp including seconds.

Inscription details lead with square artwork, then the full hash, sequence and
collection identity. Image content is contained without cropping; text content
uses Flutter's artwork background and orange ink, with a bounded text preview.
Unsupported content keeps the inscription placeholder and an explicit open-content
command. Collection icons and artwork have local fallbacks. Text load failures
remain inside the artwork and external-open failures stay in the detail body.

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
| `glass`, `glass_pill`, `glass_button` | A surface, capsule or round button that floats over the chat; never content |
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
| `slider`, `slider_compact`, `slider_media`, `slider_over_scale`, `slider_over_wave` | A value on a scale; inside a message or strip; on a dark media viewer; over a scale the caller draws; over a waveform |
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

File captions align with the attachment strip's 12pt leading inset. When the
tools hide for pending files, the shared editor adds the missing 4pt to its text
lane; plain message input keeps its tool spacing.

Pending replies sit inside the composer's rounded frame, above its attachment
tray and input row. A slim sender-colored marker, sender name and one-line preview
identify the reply; image/contact/sticker artwork uses the existing quote renderer.
An inset hairline separates it from the editor. The trailing close icon cancels
only the reply and returns focus to the existing editor. Escape retains its
composition-aware cancellation behavior. Draft text and pending files survive
reply changes; selecting the preview still locates the original message.

The emoji/sticker popover gives its content the full area above one compact footer.
Store, emoji, recent, personal and album shortcuts share that footer, following the
Flutter reference. Personal stickers begin with a dashed import tile in the same
virtual grid. The store shortcut closes the popover and opens a separate dialog.
Its catalog offers inline add/remove commands, with an unframed subscription
management command and a close icon in the header. Subscription ordering and
removal occupy a separate page in that dialog, with back navigation to the catalog.
GIF discovery is not part of the composer.


## Contact and chat pickers

Contact selection and immediate destination selection share `ui::picker` chrome:
a 480pt-wide dialog, a centered title, a leading close/back command, a pill search
field, and avatar/name rows. The list owns vertical scrolling. Contacts/chats load
in batches near the end of the list; there are no visible page controls. Search
resets the loaded result range and retains the flow's chosen identities.

Group creation is multi-select. With no choice, show only the title, search and
contact list. Once contacts are chosen, show the selected count beneath the title,
a removable horizontal avatar strip below search, and `Next` at the header's
trailing edge. The naming step retains the same header and uses `Create` there.
Keep names out of dense identity-number columns when the task is choosing people.

Message forwarding is single-select and immediate: pressing one chat/contact runs
the original forward command. It has no selection marks, selected-avatar strip,
Next action or extra confirmation step. Busy state prevents another send while the
command is running. Failures stay in the same dialog.

Recoverable failures use secondary text beside the affected content, without a red
warning block. Loading failures have one inline `Retry` text link, rather than a
separate framed button. Submission failures retain the original action and input;
retry creation from `Create`, and retry forwarding by choosing the destination.

The shared functions own appearance and geometry. Each flow still owns its query,
selection, cancellation and command; do not introduce a generic workflow controller
or give single-select destinations a multi-select presentation.


## Image viewer

The viewer reaches the window edges over a dimmed conversation. One dark header
holds the sender avatar, name and identity number, followed by zoom in/out, rotate,
forward, copy, save and close commands. Image operations do not share a floating
bottom toolbar with caption text. Captions remain optional, selectable overlays.

The image initially fits the available canvas below the header. `Mod+0` refits;
`Mod+1` selects native pixel size, accounting for display density; double-click
toggles between that size and fit. Narrow windows keep identity and close on the
first header row and place image commands on a second row.
Zoom preserves the image point under the pointer. Each axis centers when the image
fits and clamps to the viewport edges when it overflows. Resize preserves the image
point at the viewport center unless the image is fitted, in which case it refits.
Rotation preserves the viewed point and reapplies bounds and zoom limits.

On macOS, the canvas has an app-owned native input view: a mouse wheel zooms,
trackpad magnification zooms continuously, and two-finger scrolling or dragging
pans. It emits only input deltas and canvas-local pointer positions to the NSH
engine; the Rust camera remains the single owner of image geometry. Other native
backends currently retain the toolbar, keyboard and drag paths; equivalent wheel
and magnification adapters have not yet been added or validated there.

## Video viewer

Video messages share the image viewer's edge-to-edge shell: sender identity and
quiet forward, save and close commands in the dark header. Native decoding stays
in NSH's video surface with its built-in controls hidden. The video fits its canvas
without cropping. An optional caption sits above a separate transport strip.

The transport keeps playback position, elapsed/total time, mute/volume, playback
speed and full-screen mode visible. Narrow windows keep the short video header on one row and omit the volume slider
while keeping mute, seeking and speed reachable. Space toggles playback; Left/Right seek
five seconds; M toggles mute; F toggles the window's full-screen mode. Leaving a
viewer that entered full screen restores its previous window mode.

Playback events own duration, position, buffering, end and failure presentation.
Closing the modal or locking the account pauses or disposes playback. Failure stays in the canvas with a quiet retry command. Video
attachments opened from a transcript retain their transcript root for saving;
forwarding is conditional on the message policy and is omitted for transcript
children. Closing a nested video returns to the previous detail. Live streams keep their existing native-control surface.

## Forwarded chat records

Full transcripts and nested summaries use the same centered 600pt-wide dialog,
clamped to the available window. The leading close command becomes Back when a
nested record is open; equal leading/trailing slots keep the title centered.
Only the content list scrolls, without page buttons. Full records load further
100-message batches near the end, and refresh their loaded pages on message events.

Full records reuse the conversation's message rows in chronological order: sender
groups, own/incoming bubbles, date chips, media and encryption/time stamps. They
retain the transcript root for attachment operations. Conversation selection,
delivery checks, delete and recall commands do not belong to these read-only rows.

Nested summaries retain only names, categories and text. Render them through the chat row module using its incoming lane, avatar
placeholders, sender labels, bubble frame and selectable text reader. Consecutive
senders share a group; a quiet limitation note stays below the list. Do not invent timestamps, sender identities or attachments
that the summary wire format does not contain. Errors remain in the same dialog
with a quiet retry command.

## Chat information sidebar

The sidebar floats above the full-width chat as a modeless `ui::glass` surface,
inset 8pt with an 11pt radius. Opening and closing translate only the inspector;
chat rows, chrome and composer retain their geometry, including in narrow windows.
Its preferred width is 300pt, clamped to the chat's available width. Only the
inspector receives presses within its bounds; uncovered chat remains interactive.
Detail pages have no opaque root fill, so the chat supplies the material's backdrop.
Rows are drawn as the conversation list's are: inset 8pt from the panel's edges, rounded, with no
hairlines, apart only by their hover and selected fills. Space parts one section
(`parts::group`) from the next. Each page header floats over its body
(`parts::floating_page`) with glass Back or Close buttons; the body scrolls
under it past a scroll padding, and both ends fade (`ui::edge_fade`).

Rows are 44pt (`parts::group_row`), with a 12pt inner inset and a 12pt gap
between slots. All rows of
one card lead with an icon, or none do. A row that opens a page ends in a chevron.
Destructive commands are in the last card, in the danger color. Every page under
the root has Back at the leading edge of its header and a centered title; only the
root has Close.

The root starts with the identity, on the panel itself: a 90pt avatar, name with its badge and
identity/count. The description or biography is a card of its own. Search, Shared
media and Mute follow as a card of rows; an exited group keeps Mute in place,
dimmed.
There is no separate Profile row. Contact sharing, groups in common, relationship
commands and reporting are available directly in personal chats. Stranger,
blocked, friend and bot states determine the available commands; existing user
dialogs and message-avatar profile entry points retain their own navigation.

Participants, contact sharing, groups in common, shared apps and the bot's
creator form one card; disappearing messages and circles a second; name, group
photo and description editors a third; destructive commands the last. Group
photo selection opens the existing crop dialog directly. Names and descriptions
have separate editors and independent drafts; saving one does not discard the
other. The participants header exposes Add participants and Group invite link
in its trailing plus menu. Invitation links open a centered 480×600pt dialog,
clamped to the window, with the group avatar/name, a selectable full link and
three equal share/copy/reset commands. Only its body scrolls. Closing it preserves
the participants search. Reset failures remain inside the same dialog.
Non-admin members see a readable, static expiry value without a chevron, and
exited groups expose neither management nor active-chat settings.

Descriptions preview three source lines and at most 60 graphemes, with an
explicit expand/collapse command when shortened. The body owns vertical scrolling;
the 300pt inspector width and shared row controls apply to both group and personal
previews.
