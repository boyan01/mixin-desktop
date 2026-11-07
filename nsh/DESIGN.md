# Design baseline

`nsh/theme.json` is the one design baseline and `nsh/src/ui/` the one styled layer
over `nsh_base`. Values follow the Flutter app; the dark theme tracks it closely.

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
| Icon | `icon_xs` `sm` `md` `lg` | 14 16 20 24 |
| Avatar | `AvatarSize::Small` `Compact` `Bar` `Medium` `Large` | 24 32 36 40 50 |
| Height | `row_height` `nav_row_height` `list_row_height` | 52 40 80 |
| Height | `title_bar_requested_height` `button_height` `button_height_small` `field_height` | 52 36 32 40 |
| Layout | `sidebar_width` `navigation_width` `navigation_compact_width` `bubble_width` `card_width` | 300 176 64 480 260 |
| Layout | `content_max_width` `dialog_width_wide` `dialog_height_tall` | 600 640 640 |
| Layout | `sticker_panel_width` `sticker_panel_height` | 464 407 |

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
`badge_quiet`, `circle_color(id)`.

The timeline has its own roles. Bubbles take Flutter's values, and floating controls
share `bubble_in`.

| Role | Token | Light | Dark |
| --- | --- | --- | --- |
| Incoming bubble | `bubble_in` | `#ffffff` | `#343b43` |
| Own bubble | `bubble_out` | `#c5edfd` | `#3b4f67` |
| Day and notice chip | `day_background` | black 6% | white 8% |
| Chip text | `day_foreground` | `#666c78` | `#b9bdc2` |
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
| `link` | A command read inside running text, in the accent color |
| `icon_button`, `round_button` | An icon-only command in content; in a bar. Always give an accessible name |
| `field`, `textarea` | Text input |
| `search_pill`, `search_input` | The search frame and its frameless input |
| `select`, `select_trigger`, `switch` | Choice controls. Use `switch` through `toggle_row` in settings |
| `checkbox`, `radio` | An independent choice; one of a `radio_group`. `NO_LABEL` shows none: give an accessible name or add content |
| `slider`, `slider_compact`, `slider_over_scale` | A value on a scale; inside a message or strip; over a scale the caller draws |
| `tab` | One tab of a base `tabs` row, sharing its width |
| `option_row` | An option of a listbox driven from an input; never takes focus |
| `count_pill` | A count; emphasized only when it asks for attention |
| `avatar_view`, `conversation_avatar` | A person; a conversation with the group puzzle. Both take an `AvatarSize` |
| `avatar_puzzle` | The one-to-four tile puzzle, inside a clipping frame |
| `section`, `section_label` | A headed group of cards; the heading alone |
| `divider` | A hairline between groups of one pane |
| `menu`, `menu_row`, `popover` | Anchored surfaces. They and `select` bring the popup treatment; add none at the call site |
| `native_context_menu` | The OS menu on a secondary click |
| `hint` | The tooltip of a control whose label is hidden |
| `modal`, `modal_card`, `modal_title`, `scrim` | A blocking decision |
| `error_text`, `placeholder`, `empty`, `busy_indicator` | Failure, empty pane, empty timeline or list, real pending work |

`Primary` marks the one default commitment of a decision area, not the only button.
