# macOS SwiftUI parity inventory

Flutter under `flutter/` is the source of truth. This inventory follows the
runtime view tree from the application root to leaf pages. Visual parity covers
layout, size, typography, color, spacing, visible states, interactions, and
navigation. Validation for this pass is source inspection and compilation only.

Status:

- `[ ]` pending
- `[-]` source trace or implementation in progress
- `[x]` source-aligned and compiled
- `[?]` blocked by a concrete missing contract

## Root view tree

- [x] App bootstrap, lifecycle, theme, localization, and common overlays
  - Flutter: `MixinDesktopApp`, `_AppLifecycleScope`, `_AppBody` —
    `flutter/lib/app.dart`
  - Swift: `MixinMessengerApp`, `AppRootView`, `AppModel` —
    `swiftui/MixinMessenger/App/`
  - Current slice: Flutter theme tokens, launch card, recovery flows, and
    version placement are source-aligned. `./build-swiftui.sh` passes.
- [x] Authentication and account gates
  - Flutter: `LoginPage`, `QrLoginCard`, `AuthGuard`,
    `AccountHealthOverlays` — `flutter/lib/pages/login_page.dart`,
    `flutter/lib/widgets/`
  - Swift: `LoginView`, `AuthGuard`, `AccountHealthView` —
    `swiftui/MixinMessenger/Features/Authentication/`
  - Login card metrics, QR/retry/lock assets, passcode geometry, focus and
    biometric behavior, profile setup, local-time recovery, and required-update
    overlays are source-aligned. `./build-swiftui.sh` passes.
- [x] Home shell and global navigation
  - Flutter: `HomePage`, `_HomeBody`, `DesktopShellLayout` —
    `flutter/lib/pages/home_page.dart`
  - Swift: `HomeView`, `HomeNavigationModel` —
    `swiftui/MixinMessenger/App/HomeView.swift`,
    `swiftui/MixinMessenger/Navigation/`
  - Current slice: Flutter breakpoints, column widths, drawer scrim, background,
    separators, empty chat state, and route ownership are aligned.
    `./build-swiftui.sh` passes.
- [x] Sidebar
  - Flutter: `HomeSidebar` — `flutter/lib/widgets/home_sidebar.dart`
  - Swift: `HomeSidebarView`, `SidebarModel` —
    `swiftui/MixinMessenger/Features/Sidebar/`
  - Item geometry, typography, colors, spacing, badges, profile avatar fallback,
    circle actions/reorder, collapse visibility, and the collapsed interactive
    hover card are source-aligned.
- [x] Conversation list, filters, search, network status, and audio player
  - Flutter: `ConversationListPane`, `ConversationListView`,
    `ConversationSearchResults`, `NetworkStatus`, `AudioPlayerBar` —
    `flutter/lib/pages/conversation_list_pane.dart`, `flutter/lib/widgets/`
  - Swift: `ConversationListView`, `ConversationListContent`,
    `ConversationRow`, `NetworkStatusView`, `AudioPlayerBar` —
    `swiftui/MixinMessenger/Features/Conversations/`
  - List row geometry, search result sections and expansion,
    local/global/Mao/URL/identity-number search, network states, audio footer,
    mute/pin/delete, circle membership actions, localization, and leaf assets
    are source-aligned.
- [x] Conversation creation flows
  - Flutter: `conversation_create_dialogs.dart`
  - Swift: `ConversationCreationSheet.swift`
  - Search contact, single/multi selection, selected previews, group
    confirmation, circle membership creation, section geometry, identity
    filtering, and group avatar puzzles are source-aligned.
- [x] Chat header, timeline window, scroll coordination, and empty states
  - Flutter: `ChatView`, `ChatHistoryViewport`, `ChatTimelineWindow` —
    `flutter/lib/widgets/chat_view.dart`, `flutter/lib/widgets/chat/`
  - Swift: `ChatTimelineView`, `ChatTimelineStore`, `ChatScrollCoordinator` —
    `swiftui/MixinMessenger/Features/Chat/`
  - Header subtitle/action visibility, sticky day indicator and
    push-off behavior, pinned-message overlay persistence, scam-warning
    dismissal, unread/mention controls, and direct/group sender presentation
    are source-aligned.
- [x] Message rows and every message category
  - Flutter: `MessageRows`, `MessageLayout`, `MessageContent`,
    `message_items/` — `flutter/lib/widgets/`
  - Swift: `MessageContentView` and `Messages/` —
    `swiftui/MixinMessenger/Features/Chat/Messages/`
  - Grouping, avatar/name/badge rules, selection geometry,
    bubble nip and disappearing state, inline unsupported/waiting links,
    recall/location/transcript/snapshot assets, audio/file/contact/post
    metadata, media/sticker sizing, app cards, transfer/snapshot details, and
    media/post/transcript previews are source-aligned.
- [x] Composer, mentions, reply, attachments, voice, stickers, and drag/drop
  - Flutter: composer descendants in `ChatView`, `MentionController`,
    `VoiceRecorderController`, sticker pages — `flutter/lib/`
  - Swift: `ChatTimelineView`, `MentionComposer`, `VoiceRecorderBar`,
    `AttachmentPreviewSheet`, `Features/Stickers/`
  - 56-point composer geometry, text surface, placeholders,
    send/voice switching, mentions, reply preview, recording controls, and
    discard-recording confirmation, attachment preview/editor, sticker store,
    emoji categories, personal stickers, and GIF leaves are source-aligned.
- [x] Message actions, forwarding, selection, search, and detail sheets
  - Flutter: `MessageActionsMenu`, `MessageActionController`, forward/search/
    preview dialogs — `flutter/lib/widgets/`, `flutter/lib/controllers/`
  - Swift: `MessageActionHandler`, `MessageActionPolicy`,
    `ForwardConversationSheet`, `MessageSearchSenderSheet`,
    `AttachmentPreviewSheet`
  - Flutter action eligibility, single/combined forward, selection toolbar,
    pinned-message restrictions, copy/save/recall/delete/add-sticker behavior,
    search filters, QR detail, and contact/forward selectors are source-aligned.
- [x] Chat info root and nested destinations
  - Flutter: `ChatInfoPage`, `ChatSideRouter`, participants, circles, pins,
    shared media/apps, groups in common, disappearing messages —
    `flutter/lib/pages/chat_side/`
  - Swift: `ConversationInfoView` and sibling destination views —
    `swiftui/MixinMessenger/Features/ConversationInfo/`
  - Root profile metrics, cell geometry, share-link action,
    mute-until display, group-only creation date, six-line biography expansion,
    live conversation-change refresh, participants, and disappearing-message
    assets/help/custom duration, shared media/apps, circles, pins, destructive
    actions, and groups-in-common leaves are source-aligned.
- [x] User profile and protocol send flows
  - Flutter: `ShowMessageUserDialog`, `AppProtocolHandler`,
    `ShowSendMessageDialog` — `flutter/lib/widgets/`
  - Swift: `MessageUserProfileView`, `ProtocolSendSheet`
  - Message-user profile avatar fallback, background refresh, biography
    expansion, Flutter action assets, deep-link parsing/presentation, payload
    previews, destination lookup, and send behavior are source-aligned.
- [x] Settings root and every settings destination
  - Flutter: `SettingsPage`, account/preference/storage/about/MCP pages —
    `flutter/lib/pages/settings_*.dart`, `flutter/lib/pages/mcp_settings_page.dart`
  - Swift: `SettingsView`, `AccountSettingsView`, `SecuritySettingsView`,
    `ProxySettingsView`, `StorageSettingsView`, `McpSettingsView`
  - Settings list/profile metrics and the edit profile, notifications, storage
    usage/detail, security, proxy, appearance, MCP, about/log, and account
    destinations are source-aligned.
- [x] Device transfer, notifications, command palette, menus, and window actions
  - Flutter: `DeviceTransferWidget`, `HomeNotificationBridge`,
    `CommandPalette`, `HomeMacOSMenuBar` — `flutter/lib/widgets/`
  - Swift: `DeviceTransferView`, platform notification controller,
    `CommandPaletteSheet`, `AppCommands`
  - Transfer direction/setup/waiting/progress/event behavior, notification
    previews, recent/dynamic command search, result highlighting, menu grouping,
    and window navigation actions are source-aligned.

## Static validation

- [x] `./build-swiftui.sh` for the root/authentication slice
- [x] `git diff --check` for the root/authentication slice
- [x] Final `./build-swiftui.sh`
- [x] Final `cargo fmt --all -- --check`
- [x] Final `cargo test -p mixin_desktop_api -p mixin_desktop_swift`
- [x] Final `git diff --check`
- [x] Final source-path and unresolved-contract audit
