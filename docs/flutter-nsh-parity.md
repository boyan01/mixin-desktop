# Flutter desktop to Rust/NSH parity ledger

Audit date: 2026-11-06. This is a source-based migration inventory, not a completion report.

The reference is `/Users/yangbin/workspace/mixin/flutter-app`, HEAD
`7fa2d1ba5ab65f8a7b4418e685b4148ff81c09e9`. That working tree also contains
uncommitted generated localization and macOS Xcode/SPM changes; this audit did not
modify it. The destination already had uncommitted workspace manifest/lock changes
and an untracked `nsh/` implementation when this task started. Preserve them.

User scope clarification: this task migrates the complete code, UI and Rust
business capabilities. Importing an existing Flutter installation's local account,
history, media and settings is a separate future task. **L09 is DEFERRED-BY-USER**,
not a completion requirement for this code migration. Existing-data compatibility
and the risks identified in its investigation are not claimed as implemented.

Current implementation and acceptance status is tracked in the [125-ID acceptance queue](flutter-nsh-acceptance.md). The tables below preserve the initial baseline.

## Reading and acceptance rules

- `F:` means a path relative to the reference repository's `lib/` directory.
- `N:` means a path relative to this repository's `nsh/src/` directory.
- `A:` means a path relative to this repository's `mixin_desktop_api/src/` directory.
- `C:` means a path relative to this repository's `mixin_desktop_core/src/` directory.
- `MISSING`: no reachable destination UI for the reference workflow at audit time.
- `PARTIAL`: some reachable implementation exists, but observable behavior differs.
- `IMPLEMENTED-UNVERIFIED`: source shows the workflow; acceptance evidence is still missing.
- `REFERENCE-EXCEPTION`: the Flutter path is platform-excluded, unreachable, or a stub.
- `ACCEPTED` requires the coordinating agent to inspect the implementation and record
  the relevant source/build/business/runtime evidence. An agent's report or a successful
  compile alone does not establish acceptance. No row in this baseline is accepted.
- Each row's acceptance condition is a product contract. Check errors, cancellation,
  account ownership, navigation disposal, and stale request results as applicable.
- Native desktop is the target. Validate macOS first. Windows and Linux remain
  unverified until their native checks are recorded. Computer Use is not required or
  authorized by this migration request; do not add UI tests by default.
- This ledger records the initial audit. Update status only after reviewing new code;
  preserve remaining limitations rather than replacing them with a broad “done”.

## Reference layout and interaction contract

`F:ui/home/desktop_shell_layout.dart` defines the current desktop geometry:
176 pt full rail, 64 pt compact rail, 300 pt conversation list, 300 pt chat side
panel, and a 320 pt minimum content region. Main and side route switching each use
620 pt of their available content width. The rail becomes a drawer at its minimum
available width. A user collapse preference is distinct from automatic collapse.
These are the reference's current rules, not NSH architectural requirements.

```text
+------------------+----------------------------+----------------------------------+------------------+
| Account          | Search / unread / +        | Avatar / name / identity or count| Chat info /      |
|                  |                            | Search / info / bot action       | search / media   |
| All chats        | Avatar  Name         Time  +----------------------------------+                  |
| Contacts         |         Preview    Unread  | Date / sender / message / status |                  |
| Groups           |                            | Pinned / unread / mention jump   |                  |
| Bots             | Conversation context menu  |                                  |                  |
| Strangers        |                            +----------------------------------+                  |
| Circles          |                            | Quote / mention / attachment     |                  |
| Collapse         |                            | +  Emoji  [Message]  Mic / Send  |                  |
+------------------+----------------------------+----------------------------------+------------------+
   176 / 64 pt                 300 pt                       flexible                    300 pt
```

In narrow layouts, navigation and back behavior must preserve the selected chat,
message position, draft, and nested destination. Settings reuse the middle list
and main content region. Composer attachments and message actions must keep the
reference's confirmation and permission decisions.

## Startup, account, and lifecycle

| ID | Feature and Flutter source | Destination and business gap | Baseline | Acceptance condition |
| --- | --- | --- | --- | --- |
| L01 | QR login, loading, retry, expiry: `F:ui/landing/landing.dart`, `landing_qrcode.dart`, `notifier/landing_qrcode_notifier.dart` | `N:login.rs`; `A:login.rs`, `desktop.rs`; no obvious API gap | IMPLEMENTED-UNVERIFIED | Isolated account can authorize; expired QR refreshes; retry does not leave multiple authorizations; no secret is logged. |
| L02 | Saved-session restore and invalid authorization: `F:widgets/auth.dart`, `F:ui/provider/multi_auth_provider.dart` | `N:lib.rs`; `A:desktop.rs::restore_account` | IMPLEMENTED-UNVERIFIED | Valid session restores; revoked/invalid session reaches an actionable login state without an endless restore loop. |
| L03 | Account profile updates: `F:ui/provider/multi_auth_provider.dart`, `F:ui/home/slide_page.dart` | `N:home.rs`, `sidebar.rs`; `A:account.rs::profile_changes` | IMPLEMENTED-UNVERIFIED | Name/avatar/profile refresh update every visible account surface without restarting. |
| L04 | Connection status, reconnect, account health: `F:ui/home/conversation/network_status.dart`, `F:widgets/auth.dart` | `N:home.rs`; `A:account.rs::connection_status`, `account_health`, `retry_connection` | PARTIAL | Distinguish connecting, disconnected, revoked/deactivated account; retry reconnects without duplicate sessions. |
| L05 | Sign out and clear account: `F:ui/setting/setting_page.dart`, `F:account/account_server.dart` | `A:account.rs::sign_out`; no settings sign-out entry in initial UI | MISSING | Confirmed sign out drains tasks and notifications, removes saved credentials, returns to login, and cannot reopen old account content. |
| L06 | Graceful process shutdown: `F:utils/app_lifecycle.dart`, `F:app.dart` | `N:main.rs`, `runtime.rs`, `lib.rs`; `A:account.rs::shutdown` | IMPLEMENTED-UNVERIFIED | Window exit and SIGTERM shut down account services with bounded completion; relaunch can open the same isolated database. |
| L07 | Screen passcode and manual lock: `F:widgets/auth.dart`, `F:ui/setting/security_page.dart`, `F:account/security_key_value.dart` | No NSH lock screen or typed secure passcode contract identified | MISSING | Enable/confirm passcode, lock, reject wrong code, unlock; no plaintext passcode persisted as an ordinary UI setting. |
| L08 | Auto lock and biometric unlock: same as L07 plus `F:utils/authentication.dart` | Secure storage/authentication and lifecycle integration missing | MISSING | Inactivity settings lock correctly; supported macOS biometric failure/cancel preserves lock; unsupported platforms state availability honestly. |
| L09 | Flutter account data import | Initial `N:README.md` explicitly uses a separate data directory; no importer | DEFERRED-BY-USER | User explicitly separated existing local-data import from this code/UI/business migration. Do not claim existing histories/settings/credentials were imported or automatically compatible. |

## Shell, conversations, navigation, and search

| ID | Feature and Flutter source | Destination and business gap | Baseline | Acceptance condition |
| --- | --- | --- | --- | --- |
| N01 | Full/compact rail, manual collapse: `F:ui/home/slide_page.dart`, `desktop_shell_layout.dart` | `N:sidebar.rs`, `home.rs`; persistence missing initially | PARTIAL | Reference widths, active styling, counts, collapse preference and app relaunch behavior match. |
| N02 | Drawer and nested narrow routes: `F:ui/home/home.dart`, `route/major_navigator.dart`, `desktop_shell_layout.dart` | `N:home.rs`, `chat.rs`, `inspector.rs`; drawer behavior differs | PARTIAL | Narrow window can reach categories/settings, chat and nested panel; back restores prior destination and draft. |
| N03 | All chats, contacts, groups, bots, strangers: `F:ui/home/slide_page.dart`, `conversation/conversation_list.dart` | `N:sidebar.rs`, `home.rs`; `A:access.rs::ConversationAccess` | IMPLEMENTED-UNVERIFIED | Each category includes correct conversations and live unread counts; deleted or renamed data refreshes. |
| N04 | Conversation rows, draft/pin/mute/mention/verification badges: `F:ui/home/conversation/conversation_list.dart`, `F:widgets/conversation/badges_widget.dart` | `N:home.rs`, `presentation.rs`, `ui.rs`; several badges absent | PARTIAL | Correct names, participant puzzle avatars, localized preview/time, unread/muted/pinned/mention state and draft precedence. |
| N05 | Stable paging and selection: `F:ui/home/notifier/conversation_list_controller.dart`, `conversation/conversation_hotkey.dart` | `N:home.rs`; coarse event query API exists | PARTIAL | Paging does not duplicate/skip rows when events reorder data; selection and scroll do not jump unexpectedly. |
| N06 | Unread-only filter: `F:ui/home/conversation/unseen_conversation_list.dart`, `F:ui/provider/conversation_unseen_filter_enabled.dart` | `N:home.rs`; `A:account.rs::unseen_count_changes` | IMPLEMENTED-UNVERIFIED | Toggle and search respect unread filtering, live read changes, empty state and clear filters. |
| N07 | Conversation pin/unpin context menu: `F:ui/home/conversation/menu_wrapper.dart` | `A:access.rs::set_pinned`; initial list has no context menu | MISSING | Right click and keyboard menu toggle durable pin state and reorder list. |
| N08 | Conversation mute presets/unmute: same plus `F:widgets/conversation/mute_dialog.dart` | `A:access.rs::set_muted`; UI missing | MISSING | Presets preserve expiry semantics and direct/group target; cancel has no mutation; unmute refreshes badge. |
| N09 | Delete conversation with confirmation: `F:ui/home/conversation/menu_wrapper.dart` | `A:access.rs::delete_conversation`; UI missing | MISSING | Confirm deletion clears intended history/row, deselects deleted chat, preserves other conversations. |
| N10 | Create direct conversation: `F:widgets/actions/create_conversation_action.dart` | `N:create.rs`; `A:access.rs::open_user_conversation` | IMPLEMENTED-UNVERIFIED | Select contact opens/creates correct conversation, handles empty list/API failure, does not send a message. |
| N11 | Find remote user by ID/phone: `F:widgets/actions/create_conversation_action.dart`, `F:widgets/user/user_dialog.dart` | `N:create.rs`; `A:access.rs::search_user` | PARTIAL | Lookup opens reference profile actions and handles invalid/not-found users, rather than only opening chat. |
| N12 | Create group with selected contacts: `F:widgets/actions/create_group_conversation_action.dart` | `N:create.rs`; `A:access.rs::create_group` | IMPLEMENTED-UNVERIFIED | Validate name/membership, suppress duplicate submit, open created group, show service rejection. |
| N13 | Create circle: `F:widgets/actions/create_circle_action.dart` | `N:create.rs`; `A:access.rs::create_circle` | IMPLEMENTED-UNVERIFIED | Name validated; circle appears through account events; cancel/double-submit behaves correctly. |
| N14 | Rename/delete/reorder circles: `F:ui/home/slide_page.dart` | `A:access.rs::update_circle`, `delete_circle`, `reorder_circles`; UI missing | MISSING | Context rename/delete and drag reorder persist; deleted selected circle returns to usable category. |
| N15 | Circle membership batch editor: `F:ui/home/chat_slide_page/circle_manager_page.dart` | `A:access.rs::edit_circle_conversation`; UI missing | MISSING | Add/remove selected chats with reference direct-user/group semantics; updates list counts and active filtering. |
| N16 | Add/remove a chat from a circle: `F:ui/home/conversation/menu_wrapper.dart` | Same API as N15 | MISSING | Menu lists eligible circles, adds once, removes only current membership. |
| S01 | Global local conversation/contact/message search: `F:ui/home/conversation/search_list.dart` | `N:search.rs`; search APIs exist | PARTIAL | All sections, loading/error/empty, result identity and message jump work for current keyword; no stale result replacement. |
| S02 | Search section expand/filter/paging: same | Initial NSH caps at 64 users/chats and 32 global messages | MISSING | More/less and category filters can reach every matching local result, without silent cap. |
| S03 | Remote ID and `.mao` search: same, `F:ui/provider/search_mao_user_provider.dart` | `A:access.rs::search_user`, `search_mao_user`; UI missing | MISSING | Number/domain search returns resolved profile and error state matching Flutter. |
| S04 | Search URL open: `F:ui/home/conversation/search_list.dart` | No general NSH URI routing | MISSING | Recognized URL offers reference open action and routes through trusted URI handler. |
| S05 | Command palette/quick search: `F:ui/home/command_palette_wrapper.dart`, `F:widgets/actions/command_palette_action.dart` | Initial Cmd/Ctrl+K only focuses list field | PARTIAL | Reference palette keyboard navigation, selection, dismissal and direct jump operate. |
| S06 | In-chat search filters/paging: `F:ui/home/chat_slide_page/search_message_page.dart`, `notifier/search_message_controller.dart` | `N:inspector.rs`; `A:access.rs::search_messages` supports filters, UI only keyword + 64 cap | PARTIAL | Sender/type/date filters and more results preserve query; selected result locates correct message. |

## Timeline, message rendering, and actions

| ID | Feature and Flutter source | Destination and business gap | Baseline | Acceptance condition |
| --- | --- | --- | --- | --- |
| T01 | Message window, older/newer paging, scroll anchoring: `F:ui/home/chat/chat_history_viewport.dart`, `chat_timeline_window.dart`, `chat_scroll_coordinator.dart`, `F:ui/home/notifier/message_window_loader.dart` | `N:chat.rs`; explicit earlier/latest replaces reference scroll behavior | PARTIAL | Scroll-driven history preserves visible anchor; bounded memory; new messages do not steal an earlier reading position. |
| T02 | Jump to message, highlight and return latest: `F:ui/home/chat/message_jump.dart`, `chat_jump_buttons.dart`, `F:ui/provider/pending_chat_jump_provider.dart` | `N:chat.rs`; `A:access.rs::messages_around` | PARTIAL | Search/pinned/notification/transcript jump locates target, handles deleted target, and returns latest without duplicates. |
| T03 | Read marking and unread boundary: `F:ui/home/chat/chat_page.dart`, `chat_timeline_window.dart` | `N:chat.rs`; `A:access.rs::mark_conversation_read` | PARTIAL | Reads only active displayed chat at correct lifecycle; unread divider/count and offscreen incoming messages remain meaningful. |
| T04 | Mention jump and read state: `F:ui/home/chat/chat_jump_buttons.dart`, `F:ui/provider/mention_provider.dart` | `A:access.rs::unread_mention_message_ids`, `mark_mention_read`; UI missing | MISSING | Next unread mention jumps, marks only handled mentions, updates count. |
| T05 | Dates, group sender headers, avatar preferences, message statuses: `F:widgets/message/message_rows.dart`, `message_layout.dart`, `message_datetime_and_status.dart` | `N:chat.rs`, `presentation.rs`; preference bindings/status coverage incomplete | PARTIAL | Date separators, grouping, sender avatars/IDs, sent/delivered/read/failed states match data and settings. |
| T06 | Text selection, links, mentions, markdown: `F:widgets/message/item/text/text_message.dart`, `text/selectable.dart`, `F:widgets/markdown.dart` | Initial `N:chat.rs` renders selectable plain preview text | PARTIAL | Select/copy text, activate links/mentions, render formatting without exposing raw markup or executing untrusted actions. |
| T07 | Image thumbnail/preview/gallery: `F:widgets/message/item/image/image_message.dart`, `image_preview_page.dart` | `A:access.rs::image_messages_around`, attachment APIs; UI missing | MISSING | Correct aspect/thumbnail/loading/download state; gallery traversal, zoom and original file preview. |
| T08 | Video poster/playback/seek/fullscreen: `F:widgets/message/item/video/video_message.dart`, `video_preview_page.dart` | No video playback facade/backend found; initial UI is category text | MISSING | Download, poster, actual native playback, pause/seek/fullscreen and close stop resources. |
| T09 | Audio playback/waveform/speed/continuous queue: `F:widgets/message/item/audio_message.dart`, `F:utils/audio_message_player/`, `F:ui/home/conversation/audio_player_bar.dart` | `A:media.rs` + `mixin_desktop_media/src/player.rs` exist; message UI absent | MISSING | Play incoming audio, mark read, pause/resume/seek/speed/next, reflect errors and stop on account exit. |
| T10 | Files and download progress/retry/cancel/open/reveal/save: `F:widgets/message/item/file_message.dart`, `message_file_actions.dart` | `A:access.rs::AttachmentAccess`, `A:account.rs::attachment_progress`; UI missing | MISSING | Real attachment roundtrip; progress follows events, cancel/retry recovers, save/open/reveal use actual local path. |
| T11 | Sticker messages: `F:widgets/message/item/sticker_message.dart` | Sticker DTO/API exists; initial timeline only category label | MISSING | Correct asset/aspect/animation, unavailable sticker state, album/details actions. |
| T12 | Quote preview and source jump: `F:widgets/message/item/quote_message.dart` | Message DTO has quote data; timeline/composer missing | MISSING | Preview names/body/media consistently; click locates source; deleted/unavailable source handled. |
| T13 | Long post markdown and full reader: `F:widgets/message/item/post_message.dart` | `A:access.rs::send_post`; no reader | MISSING | Expand/read long post with links, copy and safe rendering. |
| T14 | Shared contact cards/profile open: `F:widgets/message/item/contact_message_widget.dart` | `A:access.rs::user_profile`; no card renderer/profile sheet | MISSING | Show referenced user, open profile, add/chat/share actions respect relationship. |
| T15 | Location card and map link: `F:widgets/message/item/location/location_message_widget.dart` | No location presentation mapping/UI | MISSING | Parse coordinates safely, show label/thumbnail where available, open correct map. |
| T16 | App/action cards and buttons: `F:widgets/message/item/action/`, `action_card/` | `A:access.rs::send_app_card`; URI/bot webview absent | MISSING | Card title/image/actions render; user activation uses original app/conversation context. |
| T17 | Combined forward transcript reader: `F:widgets/message/item/transcript_message.dart` | `A:access.rs::transcript_messages`, transcript attachment commands | MISSING | Nested message rendering and original sender/time, attachment actions and context restrictions preserved. |
| T18 | Transfer/safe transfer detail: `F:widgets/message/item/transfer/transfer_message.dart`, `transfer_page.dart`, `safe_transfer_message.dart`, `safe_transfer_dialog.dart` | `A:account.rs::snapshot_by_trace`, `snapshot_by_id`, `safe_snapshot_by_id`; UI missing | MISSING | Correct asset/amount/participants/status/memo/trace details; never initiate payment merely by rendering. |
| T19 | Inscription transfer/detail: `F:widgets/message/item/transfer/inscription_message/` | Snapshot DTO partly available; verify metadata/read model before extending facade | MISSING | Collection/item artwork/hash/amount/detail render for real inscription fixture, with unavailable metadata handled. |
| T20 | System, pin, recall, waiting, secret, stranger, unknown states: `F:widgets/message/item/{system,pin,recall,waiting,secret,stranger,unknown}_message.dart` | `N:presentation.rs` uses generic labels | PARTIAL | Each state conveys actual actor/action and available recovery; unknown payload does not crash or render as ordinary text. |
| A01 | Context menu permission policy: `F:widgets/message/message_action_policy.dart`, `message_actions_menu.dart`, `F:ui/home/chat/chat_menu_handler.dart` | Message command APIs exist; initial menu absent | MISSING | Menu choices follow message type/status/current group role and pinned/transcript context; keyboard reachable. |
| A02 | Reply: same + `F:ui/provider/quote_message_provider.dart` | `A:access.rs::send_text` quote parameter; UI missing | MISSING | Reply bar selects original; cancel clears quote; send persists correct quote and restores on failure. |
| A03 | Copy/select text: `F:widgets/message/message_actions_menu.dart` | Native selectable plain text exists; context action absent | PARTIAL | Exact chosen text copied, including reference mention rendering; media copy offered only when supported. |
| A04 | Individual forward: `F:ui/home/chat/chat_menu_handler.dart`, `F:widgets/user_selector/conversation_selector.dart` | `A:access.rs::forward_messages` | MISSING | Recipient selection and confirmation forward allowed types, handle failure without duplicate unintended sends. |
| A05 | Multi-select / combined forward / batch delete: `F:ui/home/chat/selection_bottom_bar.dart`, `F:ui/provider/message_selection_provider.dart` | `A:access.rs::combine_forward_messages`, `delete_messages`; UI missing | MISSING | Selection stable by message ID across recycling/paging; correct batch and transcript order, cancel clears selection. |
| A06 | Recall and re-edit: `F:widgets/message/item/recall_message.dart`, `F:ui/provider/recall_message_reedit_provider.dart` | `A:access.rs::recall_messages`; re-edit UI missing | MISSING | Permission/time rules match server; recalled content removed; own recalled text can be restored for editing where Flutter allows. |
| A07 | Pin/unpin with role/status checks: `F:ui/home/hook/pin_message.dart`, `F:widgets/message/message_action_policy.dart` | `A:access.rs::set_message_pinned`; no message action | MISSING | Current role/status gates command, pin list updates live, unpin removes correct target. |
| A08 | Pinned banner and full list: `F:widgets/pin_bubble.dart`, `F:ui/home/chat_slide_page/pin_messages_page.dart`, `F:account/show_pin_message_key_value.dart` | `N:chat.rs`, `inspector.rs`; no full banner lifecycle/preferences | PARTIAL | Pin updates appear without reopening chat; banner preview/dismiss, full list/jump/unpin reflect current data. |
| A09 | Add sticker/image as sticker: `F:widgets/message/message_actions_menu.dart`, `F:widgets/sticker_page/add_sticker_dialog.dart` | `A:access.rs::add_sticker`, `add_sticker_from_file` | MISSING | Allowed image/sticker adds once to personal collection; invalid media/failure is visible. |

## Composer and sticker library

| ID | Feature and Flutter source | Destination and business gap | Baseline | Acceptance condition |
| --- | --- | --- | --- | --- |
| C01 | Multiline text, IME and Enter/Shift+Enter behavior: `F:ui/home/chat/input_container.dart` | `N:chat.rs` | PARTIAL | IME confirm does not send accidentally; reference enter/newline behavior; keyboard focus survives send. |
| C02 | Durable per-conversation draft: same, `F:db/dao/conversation_dao.dart` | Initial `N:home.rs` stores session HashMap; `A:access.rs::update_draft` exists | PARTIAL | Draft survives navigation and relaunch, does not overwrite newly typed text after delayed load, and clears only after accepted send. |
| C03 | Pending send/failure recovery: `F:ui/home/chat/chat_send_outcome.dart` | `N:chat.rs`, `runtime.rs`; business jobs exist | PARTIAL | Accepted send may finish after navigation; failure preserves draft/quote; repeated click does not duplicate send. |
| C04 | Mention picker and replacement: `F:widgets/mention_panel.dart`, `F:ui/home/chat/input_highlight_controller.dart` | `A:access.rs::search_group_users`, `replace_mentions`, `mention_names` | MISSING | Typing @ opens filtered members, keyboard select inserts correct identity encoding, display name resolves. |
| C05 | File attachment picker: `F:ui/home/chat/input_container.dart`, `files_preview.dart` | `N:composer.rs::attachments` sends all files as DATA/octet-stream without preview | PARTIAL | Selected files preview with names/size; cancel sends nothing; actual attachment category/MIME correct. |
| C06 | Photo/video attachment, caption, quality mode: `F:ui/home/chat/files_preview.dart`, `files_preview_model.dart`, `image_caption_input.dart` | `A:access.rs::send_attachment` supports media metadata; preparation/inference missing in NSH | MISSING | Selected images/video show previews and metadata, compression/original choice and captions match reference. |
| C07 | Image editor crop/rotate/annotate: `F:ui/home/chat/image_editor.dart`, `image_editor_model.dart` | No equivalent | MISSING | Edit applies to outgoing media only, undo/cancel restores original, exported image metadata matches actual output. |
| C08 | Clipboard image/file paste and drag/drop: `F:ui/home/chat/chat_drop_overlay.dart`, `F:utils/system/clipboard.dart`, `input_container.dart` | No equivalent | MISSING | Paste/drop enters same preview/confirmation pipeline; invalid files fail clearly; text paste stays text. |
| C09 | Voice record, stop, preview, cancel, send: `F:ui/home/chat/voice_recorder_bottom_bar.dart`, `voice_recorder_state.dart` | `N:composer.rs::voice`, `A:media.rs`; modal differs from reference bottom bar | PARTIAL | Record real audio, waveform/duration/preview, correct encoded send; cancel/dispose cleans capture and temporary files. |
| C10 | Share contact/send post entry: `F:ui/home/chat/input_container.dart` | `A:access.rs::send_contact`, `send_post`; no equivalent chooser/editor | MISSING | Source reference entry choices, receiver and content confirmation, actual outgoing message rendered. |
| E01 | Emoji categories/search/recents/skin tones: `F:widgets/sticker_page/emoji_page.dart` | Initial NSH popover only personal stickers | MISSING | Inserts chosen emoji at cursor, category/search and recent selection remain usable by keyboard. |
| E02 | Recent/personal sticker tabs: `F:widgets/sticker_page/sticker_page.dart` | `N:composer.rs::stickers` only personal; `A:access.rs::recent_stickers` | PARTIAL | Both groups refresh after add/send/remove, correct thumbnail selection sends once. |
| E03 | Sticker album tabs/details/add/remove/order: `F:widgets/sticker_page/sticker_album_page.dart`, `sticker_store.dart` | Full album API in `A:access.rs::StickerAccess`; no UI | MISSING | Browse albums/store, inspect details, add/remove/reorder, preferences survive reload. |
| E04 | Import/remove personal stickers: `F:widgets/sticker_page/add_sticker_dialog.dart`, `sticker_store.dart` | `A:access.rs::add_sticker_from_path`, `remove_sticker` | MISSING | Pick/import valid media, reject invalid input, delete selected personal sticker and refresh. |
| E05 | GIF/Giphy search and send: `F:widgets/sticker_page/giphy_page.dart`, `F:api/giphy_api.dart` | `A:access.rs::send_remote_image` exists; no reusable Giphy business client identified | MISSING | Search/loading/paging/error and selected animated image send; network/API access stays in Rust business layer. |

## Conversation details, group administration, contacts

| ID | Feature and Flutter source | Destination and business gap | Baseline | Acceptance condition |
| --- | --- | --- | --- | --- |
| G01 | Chat info, profile metadata and live membership: `F:ui/home/chat_slide_page/chat_info_page.dart` | `N:inspector.rs` read-only name/announcement/member list | PARTIAL | Avatar/name/bio/identity/created/relationship and current group info update after events. |
| G02 | Group participants search/profile: `F:ui/home/chat_slide_page/group_participants_page.dart` | `A:access.rs::conversation_participants`, `search_group_users`; initial simple list has no actions | PARTIAL | Search all members, open user profile, identify owner/admin/current-user roles. |
| G03 | Add/remove member, promote/demote admin: same | `A:access.rs::update_participants`, `current_user_role` | MISSING | Role gates visible options, batch request semantics correct, server rejects unauthorized action safely, refreshed membership reflects mutation. |
| G04 | Edit group name/description/avatar: `F:ui/home/chat_slide_page/chat_info_page.dart` | `A:access.rs::edit_conversation` | MISSING | Authorized user updates each field without clobbering others; picker cancel preserves avatar. |
| G05 | Group invite code/share/reset/join: `F:ui/home/chat_slide_page/group_invite/group_invite_dialog.dart`, `F:widgets/conversation/conversation_dialog.dart` | `A:access.rs::resolve_code`, `join_group`, `rotate_group_invite` | MISSING | Show/copy link/QR, reset invalidates old invitation, join confirmation opens correct group. |
| G06 | Disappearing messages presets/custom/off: `F:ui/home/chat_slide_page/disappear_message_page.dart` | `A:access.rs::set_disappearing_messages` | MISSING | Role and reference duration bounds, current duration, custom validation and durable expiration behavior match. |
| G07 | Mute/clear chat/exit/delete group: `F:ui/home/chat_slide_page/chat_info_page.dart` | `A:access.rs::set_muted`, `clear_conversation`, `exit_group`, `delete_conversation` | MISSING | Distinguish clear history, exit membership and local delete; show reference confirmation and refresh selection. |
| G08 | Shared media grid, files and posts with paging: `F:ui/home/chat_slide_page/shared_media_page.dart`, `share_media/` | `A:access.rs::shared_messages`; UI missing | MISSING | Each tab filters/paginates, previews/downloads/opens, locate-to-chat reaches original. |
| G09 | Shared apps: `F:ui/home/chat_slide_page/shared_apps_page.dart` | `A:access.rs::shared_apps`, `local_shared_apps` | MISSING | Apps list reflects conversation, opens correct app profile/home with source context. |
| G10 | Groups in common: `F:ui/home/chat_slide_page/groups_in_common_page.dart` | `A:access.rs::groups_in_common` | MISSING | Correct mutual groups/count and selecting one navigates while preserving back path. |
| U01 | User/bot profile and membership/verification/deleted-user states: `F:widgets/user/user_dialog.dart` | `A:access.rs::user_profile`, `refresh_user_profile`; no equivalent sheet | MISSING | Profile displays current metadata and context-specific actions; errors/deleted account handled. |
| U02 | Add/remove/rename contact or bot: same + chat info | `A:access.rs::add_contact`, `remove_contact` | MISSING | Relationship/name update through shared Rust data, list refresh, repeat action does not duplicate. |
| U03 | Block/unblock/report: same + chat info | `A:access.rs::block_user`, `unblock_user`, `report_user` | MISSING | Confirm actual action, preserve report-and-block semantics, relationship and composer restriction update. |
| U04 | Share contact/copy user link: same | `A:access.rs::send_contact`; native clipboard/UI missing | MISSING | Correct selected recipients and user identity; copied URI opens same user. |
| U05 | Bot creator/home shortcut: `F:ui/home/chat/chat_bar.dart`, chat info | `A:access.rs::bot_creator_id`, `bot_home_uri`; webview absent | MISSING | Creator profile opens, home URL uses correct bot and source chat context. |

## Settings, platform integrations, links, and tools

| ID | Feature and Flutter source | Destination and business gap | Baseline | Acceptance condition |
| --- | --- | --- | --- | --- |
| P01 | Settings navigation/master-detail: `F:ui/setting/setting_page.dart`, `F:ui/home/route/major_navigator.dart` | Initial `N:home.rs` has profile-only settings list | PARTIAL | Reference settings categories, selected section and narrow back behavior are reachable. |
| P02 | Edit profile name/bio/avatar: `F:ui/setting/edit_profile_page.dart` | `A:account.rs::update_profile`, `update_avatar`; initial UI read-only | MISSING | Validate/submit fields, choose avatar, preserve edits on failure, profile events refresh rail. |
| P03 | Theme system/light/dark: `F:ui/setting/appearance_page.dart` | `nsh/theme.json` supports colors but no preference UI | MISSING | Chosen appearance applies immediately and persists; system follows OS when selected. |
| P04 | Show sender avatar/identity, chat font size preview: same | Ordinary settings API exists; no UI/bindings | MISSING | Setting persists and live message layout/text size reflects actual selection, not preview alone. |
| P05 | UI language: Flutter generated localization and app locale behavior | `N:i18n.rs` offers Chinese/English/system | IMPLEMENTED-UNVERIFIED | Save/reload/system locale/error rollback and every new surface translated; do not imply full Flutter locale coverage from two languages. |
| P06 | Notification preview and permission guidance: `F:ui/setting/notification_page.dart` | `A:desktop.rs::SettingsClient`; native delivery/permission UI absent | MISSING | Preference actually controls native content; permission denied state offers correct OS path. |
| P07 | Media auto-download photos/videos/files: `F:ui/setting/storage_page.dart` | Typed settings/get/set/streams in `A:desktop.rs`; UI missing | MISSING | Each toggle persists and runtime download decisions follow independently. |
| P08 | Storage totals/per-chat/category/delete: `F:ui/setting/storage_usage_list_page.dart`, `storage_usage_detail_page.dart` | `A:account.rs::storage_usage`, `conversation_storage_usage`, `clear_conversation_storage` | MISSING | Real file totals, category selection, confirmation, deletion and refreshed totals preserve messages and unrelated files. |
| P09 | HTTP proxy enable/select/add/remove/auth: `F:ui/setting/proxy_page.dart`, `F:utils/property/setting_property.dart` | `A:desktop.rs::proxy_settings`, `set_proxy_settings` | MISSING | Validate host/port, preserve credentials privately, select/disable persists, business network reconnects as reference. |
| P10 | About/version/help/legal/social/update/log directory: `F:ui/setting/about_page.dart`, `log_page.dart` | Logging facade exists; no about/log/update UI | MISSING | Accurate app/version; links open; logs accessible without exposing account secrets; update policy reflects actual distribution. |
| P11 | Screen security settings | See L07/L08 | MISSING | Settings are backed by actual lock behavior; never expose cosmetic switches. |
| P12 | Account phone/PIN/account deletion: `F:ui/setting/account_page.dart`, `account_delete_page.dart`, `F:widgets/user/change_number_dialog.dart` | Desktop `setting_page.dart` gates account entry to iOS primary session | REFERENCE-EXCEPTION | Do not implement destructive account flows as desktop parity without a reachable Flutter desktop entry and explicit product scope. |
| P13 | Backup page: `F:ui/setting/backup_page.dart` | Flutter page contains unimplemented toggles and is not linked from desktop settings | REFERENCE-EXCEPTION | Do not fabricate backup/auto-backup behavior to match a source stub; track actual device transfer separately. |
| D01 | Native notifications: `F:account/notification_service.dart`, `F:utils/local_notification_center.dart` | `A:account.rs::notification_events`, notification DTOs exist; no NSH platform delivery consumer | MISSING | Incoming eligible message creates native notification, respects active chat/mute/mention/quote/preview policy, recall dismisses. |
| D02 | Notification click navigation and cleanup: same | No equivalent | MISSING | Click focuses correct account/chat/message, handles deleted target, and account exit dismisses all owned notifications. |
| D03 | Native app/menu shortcuts: `F:widgets/window/menus.dart`, `window_shortcuts.dart`, `F:ui/home/conversation/conversation_hotkey.dart` | Initial NSH search/create shortcuts and minimal app menu only | PARTIAL | Preferences, new direct/group/circle, previous/next chat, conversation commands, help, close/hide/minimize/quit follow native reference. |
| D04 | Window state/tray/badge/single instance: `F:utils/system/windows.dart`, `tray.dart`, `F:main.dart`, `F:app.dart` | NSH native window starts; persistence/tray/badge/single-instance contract absent | PARTIAL | Restore bounds, close-to-hide semantics where reference applies, tray show/quit/unread badge, second launch does not contend for one DB. |
| D05 | External URI registration/initial/warm delivery: `F:widgets/protocol_handler.dart` | No NSH equivalent; native per-platform delivery missing | MISSING | Cold/warm `mixin:` link reaches active signed-in session; focus window; keep payload pending during login; no unconfirmed auto-send. |
| D06 | User/code/group/conversation/message URI routing: `F:utils/uri_utils.dart`, `F:utils/extension/src/regexp.dart` | `A:access.rs::resolve_code`, `join_group`, user/message queries exist; URI parser/router absent | MISSING | Correct exact route, parse failure/unknown state, invitation confirmation, target message jump and account context. |
| D07 | Send links and card actions: `F:utils/uri_utils.dart`, `F:widgets/message/send_message_dialog/send_message_dialog.dart` | Send APIs exist; URI action controller absent | MISSING | Data/category/destination parsed safely; user confirmation where reference requires; source bot/chat context cannot be replaced by current selection. |
| D08 | Transfer/payment/multisig code display and unsupported links: `F:utils/uri_utils.dart`, `F:widgets/payment/multisigs_payment_dialog.dart`, `unknown_mixin_url_dialog.dart` | Snapshot queries partially cover display; code variants/read models need audit | MISSING | Display code state/amount/asset/participants; unsupported pay/swap/markets/membership explicitly handled as Flutter; never silently execute financial action. |
| D09 | Embedded bot webview/popups/bridge/signing: `F:utils/web_view/desktop/bot_window.dart`, `bot_window_popup.dart`, `web_view_desktop.dart` | No NSH bot host or business bridge; `A:desktop.rs::http_request` alone is insufficient | MISSING | Native webview runtime, app identity/context, allowed navigation, popups, commands and bot signature boundary preserve reference trust and confirmation. |
| D10 | Device transfer/backup/restore: `F:utils/device_transfer/device_transfer_dialog.dart`, `transfer_protocol.dart`, `F:workers/device_transfer.dart` | `A:account.rs::device_transfer_events`, `device_transfer_command`; no NSH dialog | MISSING | Actual peer flow, request confirmation, progress/speed/cancel/version mismatch/recovery; isolate test data and accounts. Reference desktop menu exposes this only in debug. |
| D11 | MCP/service integration | No `lib/` Flutter MCP settings entry found in this audit; Rust has `C:runtime/mcp.rs` and `A:desktop.rs` | REFERENCE-EXCEPTION | Preserve existing shared/headless service; do not assume a new settings surface is required solely because Rust exposes it. |
| D12 | Calls/VoIP | No call/VoIP screen, service, or composer action found under Flutter `lib/`; filename “recall” is unrelated | REFERENCE-EXCEPTION | Calls cannot be a claimed migrated feature or newly required desktop parity item without source evidence/product scope. |

## Shared Rust capability boundary and cross-cutting acceptance

Most ordinary conversation, group, circle, message, sticker, attachment, user,
storage and proxy commands already exist in `A:access.rs`, `account.rs`, and
`desktop.rs`. Missing UI is not evidence that these services need replacement.
The coordinating agent owns facade changes; UI agents should request a concrete
screen-level contract before introducing another API/entity.

| ID | Contract and reference | Current owning source | Required evidence |
| --- | --- | --- | --- |
| B01 | Protocol/API, Blaze, reconnect: `F:blaze/`, `account/account_server.dart`, `workers/` | `C:core/message/blaze.rs`, `sender.rs`, `runtime/runner.rs` | Non-UI live connection, incoming/outgoing message and reconnect; errors/revocation mapped through `ClientError`. |
| B02 | Signal and encrypted messages: `F:crypto/signal/`, `crypto/encrypted/` | `C:core/crypto/`, `db/signal/` | Relevant existing tests plus isolated live encrypted roundtrip; decrypt/recovery states visible to UI. |
| B03 | Durable jobs/sync/SQLite/FTS: `F:workers/job/`, `db/`, `core/read_model/` | `C:core/model/job/`, `db/`, `runtime/` | Real SQLite contract tests for changed queries, ordering and transactions; no duplicate durable UI store. |
| B04 | Attachment upload/download/encryption | `C:runtime/attachment.rs`, `core/attachment.rs` | Real isolated file roundtrip with content identity, progress, cancel/retry and disposal, not log-only oracle. |
| B05 | Expiration/recall/pin/mentions | `C:runtime/message.rs`, `runtime/conversation.rs`, DB owners | Relevant rule/DB tests; UI updates by events and permission rules are consistently enforced. |
| B06 | Event lag, cancellation, shutdown, stale results | `A:account.rs`, `N:runtime.rs`, screen tasks | Inspect every subscription owner and version guard; disposed reads cancel, accepted user writes complete, lag triggers bounded reload. |
| B07 | Localization/theme/accessibility | `N:i18n.rs`, `ui.rs`, `nsh/theme.json` | New visible strings localized, semantic colors only, reusable styled controls, labels/focus/keyboard/dismissal and errors intact. |
| B08 | Public API errors and NSH independence | `A:lib.rs`, `error.rs`, facade modules | `ClientResult`/`ClientError`; no NSH/DAO/Tokio internals in public DTOs; compile all consumers on API change. |
| B09 | Credentials/settings/secure lock data | `C:runtime/desktop.rs`, `db/app/`, credentials owners | No credentials or QR/pairing artifacts committed/logged; durable settings from business layer; secure lock contract separately validated. |
| B10 | Platform and packaging | `nsh/Cargo.toml`, `nsh/nsh.json`, native entry | macOS native build/startup/shutdown first; Windows/Linux remain unverified until native platform checks; signing/package outcomes reported separately. |

## Integration and evidence log

Initial source audit only. No build, runtime, visual, protocol or delivery check was
performed by the inventory sub-agent. Prior claims in `nsh/README.md` are historical
claims and must not be reused as this task's acceptance evidence.

For each implementation batch record:

1. IDs changed, source inspected by the coordinating agent, and remaining mismatch.
2. Narrow business tests, formatting, static checks/build and consumer checks.
3. Required isolated runtime results, with actual state/content oracle.
4. `cargo fmt --all -- --check`, `cargo test --workspace --all-targets`,
   `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
5. macOS/Windows/Linux verification status and visual behavior still unverified.

No commit, push or worktree creation is authorized by this ledger.

## Implementation batch: conversation details and sidebar

The inventory agent implemented the following after the initial audit. These are
implementation notes, not `ACCEPTED` status changes. The coordinator must inspect
and validate the integrated result; native visual and live command outcomes were
not checked by this agent.

| Ledger IDs | Implemented destination | Remaining mismatch or evidence needed |
| --- | --- | --- |
| G01, G02, G03 | `nsh/src/conversation/mod.rs`, `info.rs`, `members.rs`: local detail/member/row event reload, first-load and explicit remote refresh, participant search, add/remove, owner admin controls, profile and direct message actions | Group ownership/roles and account disposal need isolated runtime confirmation; member list currently loads the complete coarse Rust participant query. |
| G04 | `nsh/src/conversation/info.rs::edit`: group name/announcement and shared `nsh/src/avatar.rs::pick` image preparation, `ConversationAccess::update_conversation_avatar` | Avatar preparation center-crops automatically; Flutter has an interactive crop editor. No group avatar upload roundtrip performed. |
| G05 | `nsh/src/conversation/info.rs::invite`: native QR bitmap, link copy, reset confirmation | Real invitation reset/join and link validity not tested; permission behavior requires final source/runtime review. |
| G06 | `nsh/src/conversation/info.rs::expiry`: presets, off, custom range, role-disabled entry | Custom entry currently uses total seconds up to four weeks, whereas Flutter offers five time units with unit-specific bounds. |
| G07 | `nsh/src/conversation/info.rs`: mute/unmute four reference presets, clear/exit confirmation | Current chat removal/navigation must be coordinated with session navigation after exit; group delete and UI outcomes remain to validate. |
| G08 | `nsh/src/conversation/shared.rs`: media/post/file tabs, bounded 60-row pagination, message location and thumbnail data | Uses list + previous/next pages rather than Flutter's media grid/scroll loading; standalone preview/playback relies on chat surfaces. |
| G09, G10, U01, U02, U03 | `nsh/src/conversation/user.rs`, re-exported `conversation::profile`: profile, contact name/add/remove, block/unblock/report, shared apps, groups in common, `ChatNavigation` jumps | Share-contact recipient flow and bot creator/home shortcuts still absent in this batch; localization currently Chinese/English only. |
| N15, N16 | `nsh/src/conversation/circles.rs`: per-chat circle membership and create-circle entry; `nsh/src/sidebar/circles.rs::membership_item` provides a conversation-menu entry | Per-chat membership changes are immediate. The later `nsh/src/sidebar/circle_editor.rs` batch editor uses the coordinator-provided `edit_circle_conversations` contract, re-reads current membership before save, and reconciles persisted successes after a partial failure before allowing another save. Its bounded previous/next page selector differs from Flutter scroll loading. |
| N01 | `nsh/src/sidebar/preference.rs`, `sidebar.rs`: durable collapse setting, business-runtime subscription, pending-write disabling and failure rollback | Drawer layout remains a separate shell gap; persistence and external setting updates need runtime evidence. |
| N14 | `nsh/src/sidebar/circles.rs`: native context rename/delete, move up/down, authoritative account circle stream and deleted active-circle reset | Explicit up/down replaces Flutter drag sorting. NSH 0.4.0 has pan primitives but no supported reorderable-list component found in the resolved crate source. No custom drag system was added. Collapsed category/circle icons now retain unread dots, use the `unread` semantic color, dim all-muted counts, and expose tooltips. |

At the end of this agent's first conversation/sidebar pass,
`cargo check -p mixin_desktop_ui --all-targets` passed. Intermediate Clippy runs
were blocked by compilation and unused-hook warnings while other agents were
editing and integrating files. The final workspace fmt/test/Clippy/build and live
runtime evidence belongs in the coordinator's integration log, not this snapshot.

## Implementation batch: create, global search, and quick search

This batch was source-reviewed and compiled by the inventory agent. It remains
unverified against isolated live accounts, native interaction, and real delivery.
The coordinating agent still owns acceptance.

| Ledger IDs | Flutter source → Rust/NSH implementation | Behavior and remaining limits |
| --- | --- | --- |
| N10, N12 | `lib/widgets/actions/create_conversation_action.dart`, `create_group_conversation_action.dart`, `user_selector/conversation_selector.dart` → `nsh/src/create/contacts.rs` | Default empty-query candidates now use `selectable_users_page`, with SQL filtering before LIMIT, bounded 60-row pages, IME/request guards, single-contact navigation and stable multi-selection. Group flow selects members before the 40-character name confirmation, shows the participant puzzle/count, and preserves a created group ID if opening it fails. No duplicate creation on that retry. Explicit paging differs from Flutter scrolling. |
| N11 | Search-contact entry and `lib/widgets/user/user_dialog.dart` → `nsh/src/create/lookup.rs` | Explicit ID/phone lookup, error/retry and full user profile. Does not automatically open a conversation merely because the lookup succeeds. |
| N13 | `lib/widgets/actions/create_circle_action.dart` → `nsh/src/create/circle.rs` | Name first, then optional conversation/contact selection, then create. At most 200 choices per creation, rather than an unbounded selector. If membership partially fails, retains the created circle ID and re-reads actual membership before adding remaining choices on retry. It does not create another circle or replay already persisted additions. |
| S01, S02 | `lib/ui/home/conversation/search_list.dart` → `nsh/src/search.rs`, `search/query.rs`, `search/results.rs` | Three-result section previews, More, separate contacts/conversations/messages scopes, bounded 60-row pages, offset user/conversation paging and message cursors. Uses Rust-scoped category/circle/unread queries before pagination; partial query failures preserve successful sections. Request guards prevent stale results. Navigation failure unlocks selection; a deleted message's navigation fallback also clears pending search. |
| S03 | `lib/ui/provider/search_mao_user_provider.dart`, numeric-search entry in `search_list.dart` → `nsh/src/search/remote.rs` | Debounced independent `.mao` resolution and explicit Mixin-ID lookup open the shared user profile. Query validation/completed display follows Flutter. Errors in `.mao` resolution do not erase local results. |
| S04 | URL-search entry in `search_list.dart`, `lib/utils/uri_utils.dart` → `nsh/src/search/remote.rs` | HTTP(S) external open plus Mixin HTTPS user, conversation and code navigation, including invitation confirmation/join. Embedded bot webview, payment/multisig details and unsupported Mixin actions remain distinct gaps, not delegated blindly to the OS. |
| S05 | `lib/widgets/actions/command_palette_action.dart`, `ui/provider/recent_conversation_provider.dart` → `nsh/src/search/palette.rs`, `ChatNavigation::recent_ids` | Uses resolved NSH `command` for keyboard selection and virtualized rows; shows the last five session-visited conversations for an empty query. Arrow/Enter/Escape are native component behavior; Tab and macOS Ctrl+N/P use `CommandApi::highlight`. Includes query-result paging and close/error paths. Flutter sorts the combined user/conversation result by SQL match score; current Rust queries expose no common score, so this batch lists conversations before other friend results and does not invent UI business scoring. |
| D03 | Native create shortcuts in `lib/widgets/window/menus.dart` → `nsh/src/create.rs` | Cmd/Ctrl+N and Shift+Cmd/Ctrl+N open direct/group selection and do not replace an already open creation flow. Native menu integration and remaining window/conversation shortcuts are owned by the coordinator. |

The NSH quick-search surface follows the reference's 480 pt width and 400–600 pt
height constraints. It uses app theme tokens and styled NSH controls. No screenshot,
Computer Use, or UI test was added by this agent.

Validation after the create/search integration: `cargo check -p
mixin_desktop_ui --all-targets`, `cargo clippy -p mixin_desktop_ui --all-targets
-- -D warnings`, targeted `rustfmt --check`, and `git diff --check` passed. A
subsequent source pass added the group participant preview, completed-navigation
handling, and guarded create shortcuts; the final coordinator must rerun its
workspace baseline after all agents' changes settle.

## Implementation batch: shared URL routing and send-link previews

`lib/utils/uri_utils.dart`, `lib/widgets/protocol_handler.dart`,
`lib/widgets/message/send_message_dialog/send_message_dialog.dart`, and
`lib/widgets/unknown_mixin_url_dialog.dart` are the reference. The NSH search and
message entry points now delegate to one account-bound `LinkNavigation` in
`nsh/src/links.rs`; parsing, recipient selection, payload preview and dialog
presentation live in `nsh/src/links/`. The coordinating agent explicitly provides
the session context and mounts its surface in Home. OS URL registration/delivery
and the embedded bot webview remain separate platform work.

```text
Message / search / OS URL
          |
          v
  Session LinkNavigation
          |
          +--> Profile / conversation / message
          +--> Invite code -> explicit Join
          +--> Snapshot / payment code -> read-only details
          +--> Send payload -> preview -> recipient -> explicit Send
          +--> Unsupported Mixin action -> phone QR / copy
```

| Ledger IDs | Implemented source → destination | Remaining difference / acceptance evidence |
| --- | --- | --- |
| D05, D06, S04 | URI dispatch in `uri_utils.dart` → `nsh/src/links.rs`, `links/parse.rs`; `chat/message_links.rs` and `search/remote.rs` only delegate | Mixin scheme and Mixin-host web links route users, apps, codes, conversations, messages and snapshots. The Rust facade validates direct conversation/user identity before opening and group membership after refresh. `ChatNavigation::cancel_pending` and router request versions prevent a prior link from replacing a newer result. Actual cold/warm OS delivery is not established by this module. |
| D07 | `showSendDialog` and `_SendPage` → `links/send.rs`, `links/preview.rs`, `links/recipient.rs` | Text, post, remote image, sticker, contact and app-card payloads have a preview and explicit Send. User destination wins over conversation; a text link without an explicit destination captures its originating conversation, without reading a later selected chat. Other payloads without a destination require recipient selection. Contact/sticker preview failure keeps Send disabled and permits preview retry. Recipient selection uses bounded pages/tabs rather than Flutter's combined scrolling selector. Native delivery and media encoding still need isolated live tests. |
| D08, T18 | Transfer view in `message_snapshot.rs` extracted to shared `nsh/src/snapshot.rs`; message and URL callers delegate | Shared standard/safe/trace snapshot detail rendering. Payment and multisig codes expose read-only asset, amount, state, participants, threshold and a phone QR. No financial command is executed. Generic pay/swap/markets/membership and unknown Mixin actions use phone QR/copy; they are never passed to an external URL opener. Snapshot/message link shapes extend the original Flutter dispatch and require final product review. |
| D09 | App link handling in `uri_utils.dart` → `LinkNavigation::on_open_app(AppLink)` | Refreshes the bot, resolves its home URL through the Rust facade, merges link query parameters and retains the source conversation for the native host. If no native host callback is installed, the implemented surface explicitly offers profile/browser opening. This does not establish embedded webview, bridge/signing or popup parity. |

The generic external opener accepts HTTP, HTTPS and mailto. Malformed resource
paths, unsupported schemes, malformed send Base64/JSON and invalid contact/sticker
identifiers are rejected before confirmation. Unsupported Mixin actions show the
reference's phone handoff UI. A message send always requires an explicit user
confirmation in this migration, including Flutter shortcuts that could previously
send immediately.

Nine hermetic protocol-parser contract tests cover source-conversation retention,
recipient precedence, required recipient selection for non-text payloads,
external-scheme boundaries, unknown Mixin handoff, malformed send/resource input,
conversation user-hint preservation, and valid contact/card payloads. They do not
start NSH, operate a widget, create an account or mutate a service. This batch does
not claim native interaction, send delivery, invite, snapshot-server or financial
acceptance from these tests.

The targeted link-parser run completed successfully: `cargo test -p
mixin_desktop_ui links::parse::tests --lib` — 9 passed, 0 failed, 0 ignored.
Targeted formatting and `git diff --check` passed. The build still reported an
unused old `Actions.navigation` field after delegation, and the app-host seam's
`on_open_app`/`AppLink.source_conversation` awaits native host integration; these
are integration cleanup, not a claim of a warning-free final workspace. The
inventory agent did not run the full Cargo workspace suite for this batch.

## Coordinator verification — integration batch 2026-11-06

This section records checks performed by the coordinating agent, independently
of agent completion reports. The baseline table above remains the original
inventory; a source implementation is not automatically full product acceptance.

### Shared boundaries and integration

- The coordinating agent defined the account-level Bot and ScreenLock facades,
  bounded audio/unread/search/transcript queries, batch message permissions and
  circle edits, attachment send receipts, and media preparation/editing contracts.
  NSH consumers use `mixin_desktop_api`; no screen imports a Core DAO.
- Message permission, audio-send, attachment-send and remote-image implementations
  were separated into `runtime/message/` modules. Native bot identity/signature
  checks remain in `runtime/bot/`; NSH owns native windows and trusted frame input.
- Home owns navigation, draft delivery, playback, transfer, lock state and reusable
  conversation commands. Locking disposes account presentation and bot windows.
  A separate account-owned WebKit session anchor retains account-isolated cookies
  across locking. macOS 14+ uses a persistent profile; account exit closes its
  windows and waits for data removal. Older macOS uses a nonpersistent store.
- Native menu pin/mute/delete reuse the same conversation command handle as row
  menus. Pin/mute re-read current account data; an old selected-row DTO does not
  decide the next durable state. Native menus do not duplicate business commands.

### Checks actually run by the coordinator

- An integration `cargo test --workspace --all-targets` run passed 238 tests:
  SDK 15, API 3, Core 198 plus database CLI 2, media 8, NSH 12. More changes followed
  this run; it is an intermediate checkpoint, not the final workspace result.
- Real SQLite checks passed for scoped FTS filtering before pagination/cursors,
  downloaded-audio neighbor selection before limits, stable first-unread lookup,
  and accepted-attachment failure recovery. Transcript/user paging and shared
  message permission tests were also exercised during implementation.
- A private, newly created data directory authorized mixtest t2 through the public
  API. Profile refresh and live Blaze connection succeeded. This initial fresh
  session had no conversations; it did not establish message-flow acceptance.
- The native macOS executable restored that isolated session, mounted Home and
  reached connected. SIGTERM returned exit code 0 and completed account shutdown.
  No Computer Use or screenshot evidence was used.
- After explicit user authorization for test-account messages, a separate live
  check exchanged two Signal texts between mixtest t1 and t2, uploaded/downloaded
  a PNG and an Ogg/Opus fixture, and observed READ receipts. The coordinating agent
  independently opened both actual SQLite files read-only, checked all eight
  message rows as READ, compared text hashes, and compared all four persisted
  media files against their source SHA256/length. It also checked both waveform
  values as Base64 TEXT with the expected 64 samples, MIME and 500 ms duration,
  and verified no sending jobs remain for the original four message IDs.
- The live audio check exposed a real storage bug: three attachment completion
  paths bound waveform bytes as BLOB while message rows decode Base64 TEXT. The
  coordinator fixed all three writers and added schema v30's bounded repair of
  legacy BLOB rows. Two SQLite regressions passed for the three write paths and
  v29 reopening, preserving existing text and durable job IDs. The live check then
  delivered the same original audio ID after migration, without a duplicate send
  or manual repair SQL.

### Still open

The feature tables and batch notes retain unresolved product differences, including
narrow drawers, circle drag sorting, sender/mention presentation, nested transcript
storage and gallery traversal, recall re-edit, several message actions and details,
Flutter data import, and platform-specific work. Scroll-driven bounded timeline,
image editing, appearance settings, full reference locale copy and macOS packaging
have since gained implementations; their actual evidence is recorded below.
Real microphone capture, audible quality,
Touch ID, notification delivery/click and peer device transfer have not yet been
accepted. Windows and Linux contain explicit implementation gaps as well as
unverified native behavior. None of these are covered by the intermediate test
count above.

The coordinator also ran `mixin_desktop_api/examples/media_check.rs` against the
actual generated Ogg fixture using the local CPAL output device. Playback position
advanced through the subscription clock; pause/resume, seek, speed change, natural
finish and explicit stop passed, with the original file bytes unchanged. This
checks runtime/device operation, not a human judgment of audible quality. The
playback subscription now emits clock-driven progress while playing and disposes
its timer with the subscriber, so timeline and background controls do not remain
at the initial position between control events.

### Coordinated motion policy and current integration

Motion is an NSH product decision, independent of Flutter animation behavior.
The user explicitly delegated motion design to the coordinator; Flutter is not
an animation reference. Its layout and operation contracts remain the reference.
Layout and commands continue to follow Flutter. Shared controls use 120 ms state
feedback. Icon and primary buttons press to .985 without scaling list rows.
Menus/popovers use 100 ms opacity and 4 pt entry, 80 ms exit. Modal cards use
180 ms opacity, 8 pt movement and .985 scale with a separate 140 ms backdrop;
exit lasts 120 ms. Detail panels use 180/140 ms opacity and 12 pt translation.
Chat entry fades for 100 ms. Message jump emphasis fades over 1.2 seconds and
explicit jumps use native smooth scrolling. Paging, reordering and ordinary
message batches do not replay entry animations.

The coordinator integrated shared popup constructors and busy indicators with
leaf changes, then independently added presence ownership to quick search,
creation flows, message forwarding, detail previews and the chat inspector.
Open/command state changes immediately; exit retains presentation only. Video
previews pause as soon as close is requested. Lock UI does not animate its privacy
cover; disposing the unlocked account subtree cancels all owned motion.
Reduced motion disables translation, scale and spinner rotation, with at most
80 ms fades. Native animation completion, rather than arbitrary sleeps, owns
exit removal. Visual timing and physical interactions remain unverified.

After the media lifecycle changes, the coordinating agent rebuilt and ran the
public API media_check again with the original local Ogg fixture. Real CPAL
clock, pause/resume, seek, 1.5x speed, natural completion, Idle/empty stop snapshot
and byte-for-byte unchanged source all passed. The v28/v29 database migration
fixture now models the actual Flutter schema; the coordinator reran the version
and expiration-index regression successfully against schema v30.

## Implementation batch: interactive avatar crop (P02, G04)

`nsh/src/avatar.rs` and `avatar/crop.rs` now provide one disposable avatar picker
used by profile settings and group editing. The native Canvas has a fixed square
viewport, pointer drag, 1–8× zoom slider, arrow-key positioning, reset, cancel,
preparation feedback, and save. The dialog uses the shared `motion::Presence`
policy, including owner disposal and reduced motion. No account or conversation
write is issued when the picker or crop is cancelled. Save exports the selected
pixels before the owning page submits its existing durable upload command.

```text
+--------------------------------------+
| Crop photo                           |
| Drag to move. Use slider to zoom.     |
|  +-------------------------------+   |
|  |                               |   |
|  |     Fixed square preview      |   |
|  |       Drag photo / arrows     |   |
|  |                               |   |
|  +-------------------------------+   |
|  Zoom  o-------------------------    |
|              Reset  Cancel  Save     |
+--------------------------------------+
```

Pixel decoding, EXIF orientation, source leases and export live in
`mixin_desktop_media::avatar`, behind `MediaClient::prepare_avatar` and
`encode_avatar`. Presentation keeps only crop coordinates, zoom, and lifecycle
state. The same native Canvas/Responder and `ImageEdit::Crop` contract as the
composer editor are used; the general crop/rotate/annotation editor remains
independent. PNG, JPEG, GIF, WebP and BMP sources match Flutter's file picker;
GIF avatars use the first frame. A 10 MiB source cap, 8192-pixel dimension caps
and 64 MiB decoder allocation cap are retained as explicit resource bounds.
The source-size limit and supported formats are shown beside the chooser.

Profile export retains the existing PNG/standard-base64 request contract.
Group export follows Flutter `utils/group_photo.dart`: JPEG, at most 512 pixels
per side without upscaling, unpadded URL-safe base64 for `icon_base64`.
Prepared files are normalized copies and remain leased through export. Original
files are never overwritten; cancel/disposal releases temporary sources.

Validation: `cargo test -p mixin_desktop_media avatar::tests` passed all seven
real-file tests. They check exact selected output pixels, original preservation,
lease cleanup, malformed/oversized inputs, invalid/overflowing/non-square crops,
512-pixel output, camera orientation, each supported image format, and the group
JPEG/base64url contract. `cargo check -p mixin_desktop_ui --all-targets` and
`cargo clippy -p mixin_desktop_media -p mixin_desktop_api -p mixin_desktop_ui
--all-targets -- -D warnings` passed. Formatting and `git diff --check` passed.
Final integrated workspace validation is recorded by the coordinator.
No UI tests or Computer Use were run. Native drag/zoom/focus/animation appearance,
real profile/group upload roundtrips, and Windows/Linux remain unverified. These
rows remain implementation evidence, not `ACCEPTED` status.

### Explicit NSH copy translations and reference locale gaps

The locale catalog now preserves the nine original Flutter ARB resources and
uses a separate exact-English-key supplemental catalog for 460 phrases. English
is the source key, with 3,129 explicit non-English values; reference locale
entries remain first priority. This closes the initial 299-phrase NSH-only gap
and fills absent reference translations for copy used by NSH, including the
sparse Indonesian and Malay resources. New avatar crop instructions and native
bot chrome/dialog labels are included. Taiwan and Hong Kong have distinct
static regional values; neither falls back to simplified Chinese.

The final copy source scan covered 534 distinct literal English/Chinese pairs
from Key declarations, translation calls and paired options. All resolve in
every supported non-English locale through the original ARB or supplemental
catalog. Unknown future phrases retain an explicit English fallback. This is
not a claim that arbitrary runtime/server/user content is translated.

Nine named templates cover deletion, selected messages, sending files, chat and
member counts, retry seconds, join date and minute/hour durations. Replacement
matches only caller-supplied names in the original template and appends argument
values verbatim. It does not interpret ICU or recursively expand user content.
A standalone temporary Rust harness compiled the actual language/catalog
modules and verified canonical locale aliases, missing-reference lookup, ARB
priority, static/template separation and nonrecursive argument insertion.
A JSON audit verified placeholder equality in every supplemental locale value.
`cargo check -p mixin_desktop_ui --all-targets` and targeted Rust formatting
passed on macOS during integration. No UI tests or Computer Use were run.
Native-speaker translation review, text expansion and live switching/layout
remain unverified; Windows/Linux presentation remains unverified.

The locale control retains the existing layout:

```text
Language   [ System language          v ]
           [ English                    ]
           [ Español                    ]
           [ Bahasa Indonesia           ]
           [ 日本語                     ]
           [ Bahasa Melayu              ]
           [ Русский                    ]
           [ 简体中文                   ]
           [ 繁體中文（台灣）           ]
           [ 繁體中文（香港）           ]
```

## Coordinator acceptance evidence — continued integration

The frozen inventory contains **115 product IDs and 10 business/cross-cutting
IDs, 125 total**. Its baseline labels are not current implementation statuses.
This section records the coordinator's own source inspection and live evidence;
remaining work is not accepted merely because a sub-agent completed its task.

- B04/C03/T07/T10: the coordinator executed an isolated real upload cancellation,
  then resumed the **same accepted message ID**. The upload was canceled before
  delivery; retry emitted positive byte progress and removed its progress entry
  on completion. The peer received the exact PNG, caption and 1024x1024 dimensions,
  and the sender observed READ. Independent read-only SQLite and SHA256 checks
  confirmed both persisted rows were READ/DONE, both local files matched, and no
  sending job remained. Evidence: `/tmp/mixin-nsh-cancel-retry-evidence.json` and
  `/tmp/mixin-nsh-cancel-retry-atomic-runtime.log`. The preceding harness run raced
  an automatic incoming download; its accepted ID was retained and was not
  silently resent. The harness now waits for an already active download.
- T17/A04/A05/B04: combined encrypted transcript and forwarded transcript were
  exercised with the original authorized text/PNG/Opus fixtures. The coordinator
  independently checked four READ/DONE parent rows and eight attachment files,
  including original ordering, sender/time, optional duration, caption and
  waveform. Mixed forwarding also passed after the real receiver exposed and
  the business decoder fixed dropped captions and an absent-duration-to-zero
  conversion. No historical messages were repaired with SQL.
- T07/T10/T17/A03: the coordinator executed six real ordinary/transcript exports.
  Saved bytes matched their accepted fixtures. Re-encoded PNG clipboard payloads
  decoded to identical original pixels, including both locally sent scoped
  transcript files and received legacy-basename files. Temporary exports were
  removed. Evidence: `/tmp/mixin-nsh-export-runtime.log`. Actual OS clipboard
  write/readback remains unverified.
- A08/B05: the coordinator added bounded pin pages, count/anchor position and
  transaction-backed unpin-all commands. The UI reads current conversation
  permissions; the mutation rechecks them inside its transaction. Real SQLite
  regressions cover a failure in the second job chunk rolling back all earlier
  deletions/jobs, retry without duplicate work, cross-conversation isolation and
  removed/quit membership. Incoming PIN restores a dismissed preview, while
  UNPIN preserves dismissal; a real decrypt-service/database regression checks
  this and account-key isolation. Pin queries/commands now live in
  `runtime/message/pinning.rs`, with transaction helpers under the owning DAO.
- T09/A08: the coordinator removed the duplicate chat playback observer. Chat
  and standalone media readers use Home's existing playback signal. Clicking a
  currently playing voice message stops it, following the reference operation;
  a subsequent click builds the queue from the current page, so a paused ordinary
  queue cannot bypass a pinned-only query.
- T13/T10: POST inline previews are bounded to 400 points and open one full
  scrollable reader. Downloaded `.md` attachments reuse that reader. The business
  read resolves persisted account/root identity, requires downloaded DATA `.md`,
  validates UTF-8 and reads at most 2 MiB plus a sentinel. Oversize/invalid files
  fail explicitly and can be opened with the default application. The 2 MiB bound
  is a current reader resource limit, not a protocol requirement.
- T11/E02/E03: the coordinator reviewed the shared software Lottie renderer,
  source/frame/composition cache bounds, dropped-owner request handling and real
  viewport pause wiring. It uses only ThorVG `vendored,lottie,png`; no expression
  engine or native asset-file loader is enabled. Renderer pixel tests and timing
  probes are useful implementation evidence, not visual or universal frame-rate
  acceptance. The macOS packager includes notices from the exact renderer sources.

All live message mutations above were confined to the user-authorized mixtest
peers and marked acceptance fixtures. No personal conversation, worktree, commit
or push was used. Final workspace/native checks and the remaining current-source
differences must be recorded before declaring the migration complete.
