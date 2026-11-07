# Flutter to NSH feature parity

Reference: Flutter desktop app (`/Users/yangbin/workspace/mixin/flutter-app`). 125 feature IDs; macOS is the first validation target.

Status meanings:

- **Done**: implemented in Rust/NSH source and covered by source/build/test checks; macOS native UI was exercised for main pages. Windows and Linux are not accepted as a whole.
- **Partial**: implemented, but a physical, OS-level or device check is still pending (see Note).
- **Missing**: no reachable implementation. None at present.
- **Deferred**: out of current scope (user decision or unreachable in the Flutter reference).

Remaining work and priorities live in [nsh/ROADMAP.md](../nsh/ROADMAP.md).

## Startup, account, lifecycle

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| L01 | QR login, loading, retry, expiry | Done |  |
| L02 | Saved-session restore and invalid authorization | Done |  |
| L03 | Account profile updates | Done |  |
| L04 | Connection status, reconnect, account health | Done |  |
| L05 | Sign out and clear account | Done |  |
| L06 | Graceful process shutdown | Done |  |
| L07 | Screen passcode and manual lock | Partial | physical/OS check pending |
| L08 | Auto lock and biometric unlock | Partial | physical/OS check pending |
| L09 | Flutter account data import | Deferred | separate future task |

## Navigation and conversation list

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| N01 | Full/compact rail, manual collapse | Done |  |
| N02 | Drawer and nested narrow routes | Done |  |
| N03 | All chats, contacts, groups, bots, strangers | Done |  |
| N04 | Conversation rows, draft/pin/mute/mention/verification badges | Done |  |
| N05 | Stable paging and selection | Done |  |
| N06 | Unread-only filter | Done |  |
| N07 | Conversation pin/unpin context menu | Done |  |
| N08 | Conversation mute presets/unmute | Done |  |
| N09 | Delete conversation with confirmation | Done |  |
| N10 | Create direct conversation | Done |  |
| N11 | Find remote user by ID/phone | Done |  |
| N12 | Create group with selected contacts | Done |  |
| N13 | Create circle | Done |  |
| N14 | Rename/delete/reorder circles | Done |  |
| N15 | Circle membership batch editor | Done |  |
| N16 | Add/remove a chat from a circle | Done |  |

## Search

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| S01 | Global local conversation/contact/message search | Done |  |
| S02 | Search section expand/filter/paging | Done |  |
| S03 | Remote ID and .mao search | Done |  |
| S04 | Search URL open | Done |  |
| S05 | Command palette/quick search | Done |  |
| S06 | In-chat search filters/paging | Done |  |

## Timeline and message types

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| T01 | Message window, older/newer paging, scroll anchoring | Done |  |
| T02 | Jump to message, highlight and return latest | Done |  |
| T03 | Read marking and unread boundary | Done |  |
| T04 | Mention jump and read state | Done |  |
| T05 | Dates, group sender headers, avatar preferences, message statuses | Done |  |
| T06 | Text selection, links, mentions, markdown | Done |  |
| T07 | Image thumbnail/preview/gallery | Done |  |
| T08 | Video poster/playback/seek/fullscreen | Done |  |
| T09 | Audio playback/waveform/speed/continuous queue | Done |  |
| T10 | Files and download progress/retry/cancel/open/reveal/save | Done |  |
| T11 | Sticker messages | Done |  |
| T12 | Quote preview and source jump | Done |  |
| T13 | Long post markdown and full reader | Done |  |
| T14 | Shared contact cards/profile open | Done |  |
| T15 | Location card and map link | Done |  |
| T16 | App/action cards and buttons | Done |  |
| T17 | Combined forward transcript reader | Done |  |
| T18 | Transfer/safe transfer detail | Done |  |
| T19 | Inscription transfer/detail | Done |  |
| T20 | System, pin, recall, waiting, secret, stranger, unknown states | Done |  |

## Message actions

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| A01 | Context menu permission policy | Done |  |
| A02 | Reply | Done |  |
| A03 | Copy/select text | Partial | physical selection/link gestures unverified |
| A04 | Individual forward | Done |  |
| A05 | Multi-select / combined forward / batch delete | Done |  |
| A06 | Recall and re-edit | Done |  |
| A07 | Pin/unpin with role/status checks | Done |  |
| A08 | Pinned banner and full list | Done |  |
| A09 | Add sticker/image as sticker | Done |  |

## Composer

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| C01 | Multiline text, IME and Enter/Shift+Enter behavior | Done |  |
| C02 | Durable per-conversation draft | Done |  |
| C03 | Pending send/failure recovery | Done |  |
| C04 | Mention picker and replacement | Done |  |
| C05 | File attachment picker | Done |  |
| C06 | Photo/video attachment, caption, quality mode | Done |  |
| C07 | Image editor crop/rotate/annotate | Done |  |
| C08 | Clipboard image/file paste and drag/drop | Done |  |
| C09 | Voice record, stop, preview, cancel, send | Partial | physical device check pending |
| C10 | Share contact/send post entry | Done |  |

## Emoji, stickers, GIF

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| E01 | Emoji categories/search/recents/skin tones | Done |  |
| E02 | Recent/personal sticker tabs | Done |  |
| E03 | Sticker album tabs/details/add/remove/order | Done |  |
| E04 | Import/remove personal stickers | Done |  |
| E05 | GIF/Giphy search and send | Done |  |

## Chat info and groups

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| G01 | Chat info, profile metadata and live membership | Done |  |
| G02 | Group participants search/profile | Done |  |
| G03 | Add/remove member, promote/demote admin | Done |  |
| G04 | Edit group name/description/avatar | Done |  |
| G05 | Group invite code/share/reset/join | Done |  |
| G06 | Disappearing messages presets/custom/off | Done |  |
| G07 | Mute/clear chat/exit/delete group | Done |  |
| G08 | Shared media grid, files and posts with paging | Done |  |
| G09 | Shared apps | Done |  |
| G10 | Groups in common | Done |  |

## Users and profiles

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| U01 | User/bot profile and membership/verification/deleted-user states | Done |  |
| U02 | Add/remove/rename contact or bot | Done |  |
| U03 | Block/unblock/report | Done |  |
| U04 | Share contact/copy user link | Done |  |
| U05 | Bot creator/home shortcut | Done |  |

## Settings

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| P01 | Settings navigation/master-detail | Done |  |
| P02 | Edit profile name/bio/avatar | Done |  |
| P03 | Theme system/light/dark | Done |  |
| P04 | Show sender avatar/identity, chat font size preview | Done |  |
| P05 | UI language: Flutter generated localization and app locale behavior | Done |  |
| P06 | Notification preview and permission guidance | Partial | physical/OS check pending |
| P07 | Media auto-download photos/videos/files | Done |  |
| P08 | Storage totals/per-chat/category/delete | Done |  |
| P09 | HTTP proxy enable/select/add/remove/auth | Done |  |
| P10 | About/version/help/legal/social/update/log directory | Done |  |
| P11 | Screen security settings | Partial | physical/OS check pending |
| P12 | Account phone/PIN/account deletion | Deferred | unreachable/unsupported in Flutter reference |
| P13 | Backup page | Deferred | unreachable/unsupported in Flutter reference |

## Desktop integration

| ID | Feature | Status | Note |
| --- | --- | --- | --- |
| D01 | Native notifications | Partial | physical/OS check pending |
| D02 | Notification click navigation and cleanup | Partial | physical/OS check pending |
| D03 | Native app/menu shortcuts | Done |  |
| D04 | Window state/tray/badge/single instance | Partial | physical/OS check pending |
| D05 | External URI registration/initial/warm delivery | Partial | physical/OS check pending |
| D06 | User/code/group/conversation/message URI routing | Done |  |
| D07 | Send links and card actions | Done |  |
| D08 | Transfer/payment/multisig code display and unsupported links | Done |  |
| D09 | Embedded bot webview/popups/bridge/signing | Partial | physical/OS check pending |
| D10 | Device transfer/backup/restore | Partial | peer-device transfer pending |
| D11 | MCP/service integration | Deferred | unreachable/unsupported in Flutter reference |
| D12 | Calls/VoIP | Deferred | unreachable/unsupported in Flutter reference |
