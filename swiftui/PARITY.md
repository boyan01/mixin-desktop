# macOS SwiftUI parity inventory

This file is the living implementation inventory for the native macOS target.
The product truth source is the in-repository Flutter application under
`flutter/`.

Status:

- `[ ]` pending
- `[-]` implementation in progress or only partially reachable
- `[x]` Swift implementation complete; any remaining live verification is
  recorded on the node
- `[?]` blocked by one concrete unresolved question recorded on the node

A node is complete only when every visible state and user action is reachable.
Rendering a placeholder, hiding an action, or leaving a command as a no-op does
not complete a node.

## Source entry tree

- [x] Application bootstrap and root state machine
  - Source: `MixinDesktopApp`, `_AppLifecycleScope`, `_WindowShortcuts`,
    `_AppBody`, `_SignedInBody` — `flutter/lib/app.dart`
  - Source: `main`, `_initializeDesktopWindow` — `flutter/lib/main.dart`
  - Required Rust API: `DesktopHandle`, `AccountHandle`, `SettingsHandle`
  - Swift target: `App/MixinMessengerApp.swift`, `App/AppModel.swift`,
    `Session/AccountSession.swift`
  - Contract: initialize logging/runtime, restore saved account, expose explicit
    starting/signed-out/signed-in/recovery states, initialize account-scoped
    services once, cancel them on sign-out, preserve lifecycle/focus behavior.
  - Swift implementation: `AppModel`, `AppPhase`, `AppRootView`,
    `AccountSession` — `swiftui/MixinMessenger/App/`, `swiftui/MixinMessenger/Session/`
  - Validation: `cargo test -p mixin_desktop_swift` and Xcode Debug build pass.
    A 2026-07-26 live launch restored the saved account, rendered authoritative
    circles/unseen counts and opened a real group timeline. Lifecycle/focus
    edge-case verification remains pending.
- [x] Localization, themes, text scaling, assets, and common error presentation
  - Source: `MaterialApp` — `flutter/lib/app.dart`
  - Source: `buildMixinTheme` — `flutter/lib/theme.dart`
  - Source: `SettingsController` — `flutter/lib/controllers/settings_controller.dart`
  - Source: `AppearanceSettingsPage`, `_ChatTextSizePreview`,
    `_PreviewMessage` — `flutter/lib/pages/settings_preference_pages.dart`
  - Source: `ToastWidget`, `AlertDialogLayout`, `MixinButton` —
    `flutter/lib/widgets/toast.dart`,
    `flutter/lib/widgets/mixin_dialog.dart`
  - Source: `QrLoginCard` — `flutter/lib/widgets/qr_login_card.dart`
  - Source assets: `AppIcon.appiconset`, `MixinAssets.logo`,
    `MixinAssets.aboutLogo` —
    `flutter/macos/Runner/Assets.xcassets/AppIcon.appiconset/`,
    `flutter/assets/images/logo.png`,
    `flutter/assets/images/about_logo.png`
  - Required Rust API: typed `ClientError` plus the durable `brightness`,
    `messageShowAvatar`, `messageShowIdentityNumber`, `messagePreview` and
    `chatFontSizeDelta` settings; presentation remains in Swift.
  - Swift implementation: `MixinTheme`, `SettingsPreferencesModel`,
    `MixinLocalizations`, `Localizable.xcstrings`, `MixinErrorPresenter`,
    `MixinNoticeCenter` and `MixinNoticeOverlay` —
    `swiftui/MixinMessenger/DesignSystem/Theme/`,
    `swiftui/MixinMessenger/Support/`, `swiftui/MixinMessenger/Bridge/ErrorPresentation/`.
    The Flutter light/dark semantic palette and accent are available through
    the SwiftUI environment; persisted theme is loaded before account restore
    and applies to launch, login, recovery and signed-in UI. Native macOS text
    scaling remains enabled and the independent -2...4 chat font delta is
    clamped and persisted. Typed UniFFI errors, attachment upload failures and
    common Mixin API error codes now produce stable user-facing messages.
    Feature models use the shared typed presenter instead of displaying raw
    `localizedDescription`; the presenter logs the original typed error with
    source file, function and line before lowering it for users. Notification
    delivery retains its existing raw log-only diagnostics.
    `Localizable.xcstrings` covers all 430 strings currently extracted from the
    SwiftUI target and provides values for the eight non-English variants of
    Flutter's nine locales. 149 equivalent strings reuse Flutter ARB
    translations; Swift-specific wording without an ARB equivalent has an
    explicit English fallback, so no locale renders an empty key.
    The exact Flutter macOS AppIcon set, QR login logo, About logo and chat
    background are bundled; login QR rendering also matches Flutter's `Q`
    correction level and centered logo.
  - Validation: catalog and asset JSON validation, eight-locale
    `xcstringstool compile`, source-to-catalog extraction audit and Xcode Debug
    build pass. Locale-by-locale visual/copy review and native signed-in visual
    verification remain pending.
- [x] Window lifecycle and signed-out macOS menus
  - Source: `_AppLifecycleScope`, `_WindowShortcuts`,
    `_SignedOutMacMenuBar`, `_WindowsTitleBarDivider` —
    `flutter/lib/app.dart`
  - Source: `SystemTrayWidget` — `flutter/lib/widgets/system_tray.dart`
  - Required Rust API: none for window presentation; session intents for lock
    and sign-out.
  - Swift implementation: `MixinAppDelegate`, `AppCommands`,
    `WindowActions`; the app remains alive without a visible window, Cmd-W
    hides the current window, Dock reopen and Cmd-O restore/focus it, and the
    native signed-out menu retains About/Hide/Quit/Window/Help while
    account-only settings, lock, creation and conversation actions are visibly
    disabled through focused scene values. `MixinAppDelegate` records the
    active window and first responder on application deactivation, explicitly
    clears focus, and restores that responder when the same visible window
    becomes active again.
  - Remaining: signed-out live keyboard/menu and responder-restoration
    verification.

## Authentication and recovery

- [x] QR login
  - Source: `LoginPage`, `_LoginBody`, `LandingScaffold`,
    `_VersionInfoWidget` — `flutter/lib/pages/login_page.dart`
  - Source: `QrLoginCard`, `_QrContent`, `_Loading` —
    `flutter/lib/widgets/qr_login_card.dart`
  - Required Rust API: `DesktopHandle.beginLogin`, `LoginHandle.authUrl`,
    `LoginHandle.wait`, `LoginHandle.cancel`.
  - Contract: initial request, QR display, expiration/refresh, waiting state,
    cancellation, authenticated transition, failure details, retry.
  - Swift implementation: `LoginModel`, `LoginView`, `QRCodeView` —
    `swiftui/MixinMessenger/Features/Authentication/`
  - Validation: UniFFI generation and Xcode Debug build pass; 2026-07-26 live
    macOS smoke reached the signed-out QR screen with a rendered QR code,
    version and native menu bar. Live scan, expiration and provisioning
    transitions remain pending.
- [x] Saved-login restore and abort
  - Source: `AppController.initialize`, `AppController.abortFailedLogin` —
    `flutter/lib/controllers/app_controller.dart`
  - Required Rust API: `DesktopHandle.restoreAccount`,
    `DesktopHandle.abortSavedLogin`.
  - Swift implementation: `AppModel.start`, `AppModel.abortFailedLogin`.
  - Validation: bridge unit/build coverage passes; real saved-login failure
    recovery remains pending.
- [x] Database-open recovery
  - Source: `_DatabaseOpenFailedPage`, `_DatabaseRecreateConfirmation` —
    `flutter/lib/app.dart`
  - Required Rust API: database failure detail and recreate-account-database
    command.
  - Swift implementation: `DatabaseOpenFailure`, `RecoveryView`,
    `AppModel.recreateAccountDatabase`.
  - Validation: marker parsing and destructive confirmation tests remain
    pending; Xcode Debug build passes.
- [x] Authentication guard and security unlock
  - Source: `AuthGuard` — `flutter/lib/widgets/auth_guard.dart`
  - Source: `SecurityController` — `flutter/lib/controllers/security_controller.dart`
  - Required Rust API: account identity plus durable security settings.
  - Swift implementation: `AuthGuard`, `UnlockOverlay`, `SecurityService`;
    restored accounts with a passcode start locked, content is blurred and
    interaction-blocked, and six-digit passcode or Touch ID unlocks the
    account-scoped session.
  - Validation: Xcode Debug build passes; signed-in restore, inactivity timing
    and live Touch ID verification remain pending.
- [x] Account health gates
  - Source: `AccountHealthOverlays`, `_LocalTimeError`, `_RequiredUpdate` —
    `flutter/lib/widgets/account_health_overlays.dart`
  - Source: `_ProfileSetupGate`, `_SetupNameOverlay` —
    `flutter/lib/pages/home_page.dart`
  - Required Rust API: connection health, required-update stream,
    update-account, reconnect.
  - Swift implementation: `AccountHealthView`, `ProfileSetupView`,
    `LocalTimeErrorView`, `RequiredUpdateView` plus the cancellable
    `SwiftAccountHealthSubscription`.
  - Validation: bridge unit and Xcode Debug build pass; forced-health-state live
    validation remains pending.

## Home shell and navigation

- [x] Responsive native shell
  - Source: `HomePage`, `_HomeBody`, `DesktopShellLayout` —
    `flutter/lib/pages/home_page.dart`
  - Required Rust API: account profile and account-wide event streams.
  - Contract: drawer/compact/full sidebar modes, conversation list, chat or
    settings detail, optional chat inspector, selection preservation.
  - Swift implementation: `HomeView`, `ResponsiveHomeShell`,
    `HomeNavigationModel`, `HomeSection`; the shell uses Flutter's 64/176-point
    rail widths and 620-point main-route threshold, supports user-collapsed and
    automatic compact rails, presents a dismissible sidebar drawer at the
    narrow breakpoint, and switches the main route between conversation list
    and chat with an explicit back action. The window-scoped navigation owner
    remains stable while resizing, so conversation selection, draft and
    inspector presentation state survive layout-mode changes.
  - Validation: Swift parse, whitespace validation and the final clean Xcode
    Debug build pass. The 2026-07-26 live smoke rendered the three-column shell
    with real account data and selected a real group. The signed-in interaction
    pass also verified full, compact and drawer layouts, drawer dismissal after
    navigation, route-mode chat selection and the explicit back action.
    Inspector interaction remains pending because opening it interrupts the
    local Computer Use accessibility service while the app stays healthy.
- [x] Sidebar profile, categories, unseen badges, circles, reorder and menus
  - Source: `HomeSidebar`, `_ProfileItem`, `_CategoryItem`, `_SidebarItem`,
    `_SidebarBadge` — `flutter/lib/widgets/home_sidebar.dart`
  - Required Rust API: `circles`, `unseenCountChanges`, create/update/delete/
    reorder circle, edit circle conversations.
  - Swift implementation: `HomeSidebarView`, `SidebarModel`,
    `CircleNameSheet`, `CircleConversationsSheet`,
    `SwiftCircleSubscription`, `SwiftUnseenCountSubscription`; native drag
    reorder, context-menu rename/edit/delete, confirmed deletion, and
    conversation assignment diffing are wired to the Rust account handle.
  - Validation: `cargo test -p mixin_desktop_swift` and the signed Xcode Debug
    build pass. Signed-in pointer and accessibility activation switch All
    Chats, Contacts, Groups and a real circle in full and drawer layouts.
    Reorder and destructive circle actions remain pending.
- [x] Typed navigation and side destinations
  - Source: `_ChatWithSide` — `flutter/lib/pages/home_page.dart`
  - Source: `ConversationInfoDestination` —
    `flutter/lib/pages/conversation_info_destination.dart`
  - Required Rust API: destination-specific account access handles.
  - Swift implementation: `HomeSection`, `HomeNavigationModel`,
    `ChatInspectorRoute`, `ConversationInfoView`; the selected chat owns a
    native inspector with a typed, conversation-resetting nested path shared by
    the header and macOS Conversation menu. Participants, circles, pinned
    messages, shared content/apps, groups in common, disappearing messages and
    user/developer profiles push inside the inspector. Message search closes
    the inspector and focuses the shared chat search flow. Hidden inspector
    content is not constructed, keeping the main window title at `Mixin` while
    a conversation is selected.
  - Validation: Swift syntax parse and `cargo check -p mixin_desktop_swift`
    pass; collapsed-window and signed-in inspector restoration remain pending.
- [x] Command palette
  - Source: `_CommandPalettePage`, `_PaletteSearchField`, `_PaletteItem` —
    `flutter/lib/widgets/command_palette.dart`
  - Required Rust API: screen-ready user/conversation search.
  - Swift implementation: `CommandPaletteSheet`, `CommandPaletteModel`; Cmd-K
    opens a focused native palette, searches bounded local conversations and
    selectable users, supports list selection/Return, and creates or opens the
    selected conversation through the account handle. Escape dismisses the
    palette, while regular conversation search preserves its clear-then-unfocus
    Escape behavior.
  - Validation: signed Xcode Debug build and signed-in Cmd-K/Escape keyboard
    verification pass. Final selection/Return comparison remains pending.
- [x] Signed-in macOS menus and shortcuts
  - Source: `MacosMenuBar`, `_MacosMenuBarState` —
    `flutter/lib/widgets/home_macos_menu_bar.dart`
  - Required Rust API: navigation and active-conversation commands.
  - Swift implementation: `AppCommands`; preferences, quick search,
    new conversation/group/circle, message search, mute/unmute, pin/unpin,
    confirmed delete, previous/next conversation, zoom and help links are
    active. Chat-info toggling opens the native inspector. Shift-Cmd-L invokes
    the account security owner and is enabled whenever a passcode exists.
  - Validation: Swift bridge test and Xcode Debug build pass; signed-in menu
    invocation and keyboard navigation remain pending.
- [x] Protocol/deep-link routing
  - Source: `AppProtocolHandler`, `_AppProtocolHandlerState` —
    `flutter/lib/widgets/app_protocol_handler.dart`
  - Source: `MixinUriExtension`, `MixinSchemeHost` —
    `flutter/lib/utils/mixin_uri.dart`
  - Source: `showSendMessageDialog`, `_SendPage`, `_PayloadPreview` —
    `flutter/lib/widgets/show_send_message_dialog.dart`
  - Source: `showConversationSelector`, `_ConversationSelector` —
    `flutter/lib/widgets/show_forward_conversation_selector.dart`
  - Required Rust API: code resolution, conversation/user/snapshot lookups,
    message send commands and selectable conversations/users.
  - Swift implementation: `MixinDeepLink`, `HomeNavigationModel.open`; the app
    registers the `mixin` URL scheme, opens conversation links, sends the
    optional `start` text through the real message command, resolves identity
    number or UUID user links, resolves user/group/payment/multisig codes,
    joins or opens group codes, opens snapshot trace details, and either opens
    app profiles or launches app homepages with passthrough query parameters
    in the native bot web window. `ProtocolSendSheet`, `ProtocolSendModel` and
    `ProtocolSendService` parse Base64 text/image/sticker/contact/post/app-card
    payloads; honor explicit user and conversation destinations; preserve
    current-conversation text sending; preview payloads; search all
    conversations and selectable contacts; open direct conversations; and run
    real message commands. Protocol images are downloaded, decoded for
    dimensions and sent as temporary `IMAGE` attachments. Pay/swap/market/
    membership URLs preserve Flutter's explicit unknown-link presentation.
  - Validation: the final Rust bridge compile, regenerated UniFFI bindings and
    clean Xcode Debug build pass. Signed-in protocol-link interaction remains
    pending.

## Conversations and search

- [x] Conversation list loading and selection
  - Source: `ConversationListView`, `_ConversationListBody`,
    `ConversationItem` — `flutter/lib/widgets/conversation_list_view.dart`
  - Required Rust API: `ConversationAccess.conversationItems`,
    `conversationItemsByIds`, `conversationChanges`.
  - Swift implementation: `ConversationListView`, `ConversationListModel`,
    `ConversationRow`, `ConversationPreview`, `ConversationIdentityBadge`,
    `ConversationAvatar`, `SwiftConversationSubscription`; 50-row lazy paging,
    request-version guards, full reload recovery and targeted
    `conversationItemsByIds` event merging preserve the Rust query ordering and
    active category/circle/unseen/search filters. Rows render group avatar
    puzzles, membership/verified/bot/scam badges, drafts, outgoing status,
    sender/action/category-aware previews, mentions/unseen counts and
    mute/pin state. The Flutter 64-point search/action toolbar and 78-point row
    interaction are mirrored with a capsule search field, clear/Escape
    behavior, 36-point action targets and animated hover, pressed and selected
    feedback. Conversation rows are semantic buttons, preserving keyboard and
    accessibility activation; selectable rich-text labels do not participate
    in hit testing, so the row button exclusively owns click and drag gestures.
    Conversation change events use the same 16 ms merge window as Flutter,
    coalesce duplicate IDs and full reloads, and publish one ordered array
    mutation per batch. Navigation dictionaries also skip equal assignments,
    avoiding whole-shell invalidation on unchanged event payloads.
  - Bridge: `SwiftConversationListItem` preserves the existing facade's full
    `ConversationListData` contract instead of maintaining a second Swift-side
    data source.
  - Validation: bridge format/check and Xcode Debug build pass. The 2026-07-26
    live smoke rendered bot/group rows, verified/bot/mute badges, previews and
    unseen counts, and selection reduced the authoritative unseen total.
    The 2026-07-26 interaction pass also built a locally signed sandboxed Debug
    app, restored the existing account from the shared app container, and
    verified pointer-driven search input/clear, unseen filtering and
    conversation selection against live data. The later regression pass
    switched ten live conversations in succession, paged the conversation list
    to its lower window and back to the top, and kept click and scroll input
    responsive while high-volume conversation events continued.
- [x] Conversation category/circle/unseen filtering
  - Source: `HomeNavigationController` —
    `flutter/lib/controllers/home_navigation_controller.dart`
  - Source: `ConversationListStore` —
    `flutter/lib/controllers/conversation_list_store.dart`
  - Required Rust API: conversation list DTOs and unseen streams.
  - Swift implementation: typed `HomeSection` filters, circle IDs and
    unseen-only toggle in `ConversationListView`.
  - Validation: Xcode Debug build and live Contacts, Groups, circle and
    unseen-only toggle/restoration pass.
- [x] Conversation search and global user search
  - Source: `_SearchBar`, `_ClearSearchIntent` —
    `flutter/lib/widgets/conversation_list_view.dart`
  - Source: `ConversationSearchResults`, `_SearchHeader`, `_SearchItem`,
    `_SearchEmpty` — `flutter/lib/widgets/conversation_search_results.dart`
  - Source: `_SearchUserDialog` — `flutter/lib/pages/home_page.dart`
  - Required Rust API: local conversation search and remote user search.
  - Swift implementation: debounced local keyword search, Cmd-F focus,
    highlighted conversation name/preview matches and a real remote user
    result are wired. Selecting the remote user opens the authoritative direct
    conversation. The command palette shares the same local destinations.
  - Validation: combined bridge/Xcode validation and signed-in local search,
    exact-result rendering and clear restoration pass. Remote search comparison
    remains.
- [x] Conversation context menu and create flows
  - Source: `_ConversationContextMenu`, `ConversationCreateAction` —
    `flutter/lib/widgets/conversation_list_view.dart`
  - Source: `_NewGroupConfirm` — `flutter/lib/pages/home_page.dart`
  - Required Rust API: pin, mute, delete, create group/circle/conversation.
  - Swift implementation: conversation rows expose real pin/unpin, timed
    mute/unmute and confirmed delete operations through
    `SwiftAccountHandle`.
  - Swift implementation: `ConversationCreationSheet`,
    `ConversationCreationModel`; searchable contact single-selection opens a
    direct conversation, multi-selection creates a named group, and a named
    circle can be created with zero or more selected conversations. Conversation
    and group search merges a debounced remote user lookup with local
    selectable contacts.
  - Validation: combined bridge/Xcode validation, per-flow badges/sections and
    create-flow live verification remain.
- [x] Conversation network and audio bars
  - Source: `NetworkStatus` — `flutter/lib/widgets/network_status.dart`
  - Source: `AudioPlayerBar`, `_PlaybackSpeedButton`, `_ProgressBar` —
    `flutter/lib/widgets/audio_player_bar.dart`
  - Required Rust API: network state and attachment/audio operations.
  - Swift implementation: `NetworkStatusView` subscribes to the authoritative
    Blaze connection stream, preserves the source first-connect behavior and
    exposes the real retry command. `AudioPlayerBar` follows the shared native
    audio coordinator, stays hidden for the selected conversation, and provides
    conversation navigation, play/pause, 1x/2x, stop and progress controls.
  - Validation: combined UniFFI generation, Xcode build and signed-in
    disconnect/cross-conversation playback smoke remain pending.

## Chat timeline and message reading

- [x] Chat root, header, banners, and jump controls
  - Source: `ChatView`, `_ChatHeader`, `_HeaderAction`,
    `_ScamWarningBanner`, `_JumpCurrentButton`, `_JumpMentionButton`,
    `_PinMessagesBanner` — `flutter/lib/widgets/chat_view.dart`
  - Required Rust API: conversation detail, pin preview, read/mention anchors.
  - Swift implementation: `ChatTimelineView` provides the selected conversation
    header, loading/empty/error states and explicit refresh. Scam warnings,
    exited-group send blocking, a dismissible pinned-message preview with
    locate, unread-mention count/jump/clear controls and scroll-distance
    jump-to-latest visibility use authoritative conversation/message state.
    The header and preview both open the real pinned-messages collection,
    whose rows can locate the exact message back in the timeline.
- [x] Incremental timeline, paging, day chips, unread separator, live changes
  - Source: `_MessageList`, `_UnreadMessageBar`, `_ChatMessage` —
    `flutter/lib/widgets/chat_view.dart`
  - Source: `MessageRows` — `flutter/lib/widgets/message_rows.dart`
  - Source: `MessageDayTime`, `MessageDayTimeViewportWidget` —
    `flutter/lib/widgets/message_day_time.dart`
  - Required Rust API: `messages`, `messagesAround`, ID windows,
    `messageItemsByIds`, `messageChanges`.
  - Swift implementation: `ChatTimelineModel`, `ChatTimelineStore`,
    `ChatScrollCoordinator`, `ChatTimelineRenderBoundary`,
    `SwiftConversationSubscription` and `SwiftMessageSubscription`; the native
    `ScrollView`/`LazyVStack` timeline starts with 60 messages and loads
    bidirectional 100-message pages without a presentation-count cap while the
    conversation remains open. Stable message IDs, `ScrollPosition`,
    `ScrollGeometry`, `ScrollPhase` and row geometry preserve the visible
    anchor across prepends, row-height changes, live inserts, explicit message
    jumps and conversation restoration. Loaders stay outside scroll content,
    pagination starts three viewports before an edge, and tail following is
    conditional on the user's current position. The store incrementally
    rebuilds affected rows and mention lookups, while media/audio indexes use
    compact revisions; change events refresh visible or mutable IDs and
    active-conversation events merge the recent tail. Unseen conversations open
    around the last-read message with an unread separator and calendar day
    chips. `ScrollPosition` remains native SwiftUI state while the coordinator
    owns only scroll policy and ignores callbacks after its view disappears;
    rapid conversation replacement therefore cannot synchronously invalidate
    the outgoing message tree from a scroll-state writeback.
  - Validation: signed and unsigned Xcode Debug builds pass. The 2026-07-26
    signed-in smoke opened a real high-volume group, exercised multiple older
    page loads from 22:34 back to 21:54 without losing the visible anchor,
    switched conversations and restored the same 21:54 viewport, then returned
    to the 23:26 tail through jump-to-latest. A follow-up sample reproduced and
    removed the rapid-switch `ScrollToScrollStateRequest` invalidation loop;
    ten consecutive live switches completed and the process returned to idle.
- [x] Message layout, bubble, sender, metadata, selection and actions
  - Source: `MessagePresentation` —
    `flutter/lib/widgets/message_presentation.dart`
  - Source: `MessageBubble`, `MessageBubbleHighlight` —
    `flutter/lib/widgets/message_bubble.dart`
  - Source: `MessageDatetimeAndStatus`, `MessageStatusIcon` —
    `flutter/lib/widgets/message_datetime_and_status.dart`
  - Source: `MessageName`, `SelectableMessageText`,
    `MessageActionPolicy`, `MessageActionCallbacks` —
    `flutter/lib/widgets/message_name.dart`,
    `flutter/lib/widgets/message_selectable_text.dart`,
    `flutter/lib/widgets/message_action_policy.dart`,
    `flutter/lib/widgets/message_actions_menu.dart`
  - Required Rust API: message DTO, status transitions, recall/delete/pin.
  - Swift implementation: `MessageRow` renders incoming/outgoing alignment,
    sender, quote preview, time, delivery status and pinned metadata.
    `MessageActionPolicy` and the native context menu now gate reply, text or
    caption copy, selection, pin/unpin, recall and local delete by the same
    category, completion, role, sender and 60-minute rules as Flutter.
    Multi-message selection exposes copy and confirmed delete/recall commands.
    Entering selection mode animates a per-row selection indicator and the
    bottom selection toolbar replaces the composer with the same compact
    motion used by Flutter. Shared action-button styles expand header and
    composer icon targets and add hover/pressed feedback while respecting
    Reduce Motion.
    Image clipboard and Save As are wired through the rich-media viewer.
    Sticker clicks open their detail, and single, batch and combined forward
    actions use the real destination selector and message commands.
    `MessageQRCodeSheet` implements Flutter's text-message Generate QR Code
    action, and completed local images additionally expose native Vision QR
    scanning with copy and URL/Mixin action handoff.
- [x] Core message types
  - Source: `MessageContent`, `_TextMessage`, `_ImageMessage`,
    `MessageImage`, `_VideoMessage`, `MessageVideo`, `_StickerMessage`,
    `MessageFile`, `MessagePost`, `MediaStatusOverlay` —
    `flutter/lib/widgets/message_content.dart`
  - Source: `AttachmentStatusPending`, `AttachmentStatusDownload`,
    `AttachmentStatusUpload`, `AttachmentStatusWarning` —
    `flutter/lib/widgets/attachment_status.dart`
  - Source: `AudioMessageWidget` —
    `flutter/lib/widgets/message_audio.dart`
  - Required Rust API: attachment paths/status/progress and typed content.
  - Swift implementation: `RichMessageContent`, `MessageMediaImage`,
    `AttachmentStatusOverlay`; image/video/file records resolve the Rust-owned
    local media path, render embedded thumbnail fallback, map
    canceled/pending/expired/done status, poll real runtime progress only while
    pending, and invoke retry/download/cancel through `SwiftAccountHandle`.
    Files expose open, Save As and Finder actions using native macOS panels.
    Audio messages additionally render waveform/duration, share one native
    playback coordinator and expose seek/speed/read transitions. Static and
    animated sticker loading, cache, retry and lifecycle-aware playback are
    provided by the sticker implementation below.
- [x] Special message types
  - Source: `WaitingMessageItem`, `QuoteMessagePreview`,
    `ContactMessageItem`, `LocationMessageItem`,
    `AppButtonGroupMessageItem`, `AppCardMessageItem`,
    `TranscriptMessageItem`, `SnapshotMessageItem`,
    `SystemConversationMessageItem`, `PinMessageItem`,
    `SecretMessageItem`, `StrangerMessageItem`, `InscriptionContent` —
    `flutter/lib/widgets/message_items/special_message_items.dart`
  - Required Rust API: compatible payload decoding, snapshot/code/user lookup.
  - Swift implementation: contact, transfer and system message summaries plus
    quote rendering are wired. `AppMessageView` decodes `APP_CARD` and
    `APP_BUTTON_GROUP`, renders their header/action layouts, and routes input,
    Mixin protocol, and web actions through the real command/WebView path.
    `SpecialSnapshotMessageCard` now renders and
    opens account snapshot, SAFE snapshot and SAFE inscription cards using the
    joined message/snapshot/inscription fields from Rust. `LocationMessageItem`
    is implemented by `SpecialLocationMessageView` with a native MapKit
    coordinate preview and real Google Maps external action. `SecretMessageItem`
    is implemented by `SpecialSecretMessageView` and opens the same Mixin
    end-to-end encryption documentation URL. `PinMessageItem` is implemented by
    `SpecialPinMessageView`; it decodes the nested message ID/category/content,
    generates text/media/post/card/button/call/recall previews and applies the
    timeline's authoritative mention-name map. `StrangerMessageItem` is
    implemented by `SpecialStrangerMessageView`; sender relationship/app/bot
    fields now cross UniFFI, FRIEND/BLOCKED rows disappear, and Block, Add
    Contact, Say Hi and Open Homepage call the real Rust user/message/bot
    operations with progress and recoverable errors. These four standalone
    categories also bypass reply/selection/pin/delete menus like Flutter.
    `WaitingMessageView` presents FAILED messages as encrypted-session waiting
    rows with the sender/linked-device subject and support link. UNKNOWN,
    malformed location/transcript/app payloads and invalid image metadata use
    the same visible unsupported-message recovery state as Flutter. Shared
    contact rows preserve user ID, avatar, verification, bot and membership
    metadata, and clicking a valid row resolves the user through the shared
    deep-link/profile navigation path.
  - Validation: final `cargo fmt --all -- --check`, Swift bridge tests/clippy,
    regenerated bindings and the clean Xcode Debug build pass. Live map launch,
    relationship mutations, shared-contact navigation and bot homepage
    interaction remain pending for signed-in verification.
- [x] Rich text and Markdown
  - Source: `CustomText`, `CustomSelectableText`, `TextMatcher`,
    `UrlTextMatcher`, `MailTextMatcher`, `EmojiTextMatcher` —
    `flutter/lib/widgets/high_light_text.dart`
  - Source: `SelectableMessageText`, `MentionTextMatcher`,
    `BotNumberTextMatcher` —
    `flutter/lib/widgets/message_selectable_text.dart`
  - Source: `MarkdownControllerCache`, `MarkdownColumn`, `Markdown` —
    `flutter/lib/widgets/post_markdown.dart`
  - Required Rust API: none beyond message payloads.
  - Swift implementation: `PostMessagePreview` renders post Markdown with
    native selectable text and link handling. Reusable `MessageRichText` uses
    the native macOS link detector plus the Flutter email expression, emits
    clickable URL and `mailto:` attributes, routes clicks through an injectable
    URL action, keeps text selectable, applies Apple Color Emoji per grapheme,
    and enlarges messages containing only one to three emoji. Plain timeline
    text, compact search rows, image captions and rich-content fallbacks use the
    component. `ChatTimelineModel` batches the currently loaded timeline,
    search and pinned windows' content, caption and quoted content into one
    `mentionNames(contents:)` bridge request. Resolved `@identityNumber`
    mentions render as `@displayName` while retaining their original identity
    in the Mixin user deep link; unresolved mentions and `7000……` bot numbers
    use that same identity deep link. `HomeNavigationModel` resolves those
    identities through the real account lookup, opens their direct
    conversation and presents the native profile inspector.
  - Remaining: verify the signed-in link interactions.
  - Validation: the reusable source passes standalone macOS Swift type-check
    and the combined unsigned Debug Xcode build passes; signed-in link
    interaction remains pending.
- [x] Image, video, post and transcript previews
  - Source: `ImagePreviewPage`, `_PreviewBar`, `_PreviewImage`,
    `VideoPreviewPage`, `PostPreviewPage`, `openOrSaveMessageFile`,
    `saveMessageFileAs` —
    `flutter/lib/widgets/message_media_preview_pages.dart`
  - Source: `TranscriptMessageItem` —
    `flutter/lib/widgets/message_items/special_message_items.dart`
  - Source: `TranscriptPage`, `_TranscriptMessage` —
    `flutter/lib/widgets/transcript_page.dart`
  - Source: `MessageVideo`, `_QuoteContent` —
    `flutter/lib/widgets/message_content.dart`,
    `flutter/lib/widgets/message_items/special_message_items.dart`
  - Required Rust API: file resolution/download and attachment state.
  - Swift implementation: `ImageMessagePreview` supports the loaded
    conversation image sequence, previous/next navigation, zoom, rotation,
    clipboard copy and Save As. Approaching either sequence boundary calls the
    image-specific Rust query around the current message, merges/deduplicates
    the expanded window and preserves the selected image. `VideoMessagePreview`
    plays resolved local or HTTP media with `AVPlayer`; `PostMessagePreview`
    renders selectable native Markdown; `TranscriptMessagePreview` loads real
    transcript rows and routes transcript attachment retry/download/cancel
    through Rust. Transcript rows preserve sender/avatar/time presentation,
    locate and highlight quoted messages, and provide nested image, video/live,
    audio, file, post, sticker, contact, location, transcript-summary and app
    card/button rendering with the relevant open, save, playback and attachment
    actions. `PLAIN_LIVE` and `SIGNAL_LIVE` now follow Flutter's `_LIVE`
    classification: completed streams open the native player and active remote
    stream URLs open externally when no attachment state is present.
  - Validation: UniFFI generation and the new Swift files compile in the Xcode
    build; 2026-07-26 transcript/live narrow unsigned Debug Xcode build passes.
    Signed-in media interaction, live-stream URL/player behavior and transcript
    message-change refresh remain pending.
- [x] Snapshot, inscription and multisig dialogs
  - Source: `_SnapshotDetailPage`, `_InscriptionDetailPage`,
    `_SnapshotDetailLoader`, `_SnapshotValuesDescription`,
    `_TransactionDetailInfo`, `_SafeTransactionDetailInfo`,
    `SymbolIconWithBorder` —
    `flutter/lib/widgets/show_snapshot_detail_dialog.dart`
  - Source: `SnapshotMessageItem`, `_SafeSnapshotCard`,
    `InscriptionContent`, `_TextInscriptionContent`, `_SnapshotData` —
    `flutter/lib/widgets/message_items/special_message_items.dart`
  - Source: `_PaymentDialog`, `_PaymentBody`, `_UsersLayout`,
    `_QrCodeLayout`, `_DoneLayout` —
    `flutter/lib/widgets/show_multisigs_payment_dialog.dart`
  - Required Rust API: snapshot detail and code resolution.
  - Swift implementation: `SpecialSnapshotMessageCard`,
    `SnapshotDetailDialog`, `SnapshotDetailModel`,
    `MultisigPaymentDialog`; message cards render cache-first joined fields,
    ordinary and SAFE snapshots call the real `snapshotById` or
    `safeSnapshotById` operation, and dialogs expose explicit refreshing,
    content, failure and retry states. Account/SAFE transaction rows,
    current/then fiat values, pending confirmations, inscription image/text
    content and metadata are rendered. `mixin://snapshots?trace=...` uses
    `snapshotByTrace`; code links use `resolveCode` and present participant,
    asset, state, QR and Done layouts for payment/multisig requests. Local
    payment/signing remains visibly disabled because Flutter delegates that
    authorization to a compatible wallet through the QR link.
  - Validation: `cargo test -p mixin_desktop_swift`, final UniFFI regeneration
    and the clean aggregate Xcode Debug build pass. Signed-in snapshot lookup,
    inscription network content and wallet QR handoff remain to be verified
    live.

## Composer and message commands

- [x] Text composer, focus, shortcuts, drafts and send outcomes
  - Source: `ChatInputBar`, `_ChatInputBarState`, `_SendInputIntent`,
    `_SendPostInputIntent`, `_ChatInputAction` —
    `flutter/lib/widgets/chat_view.dart`
  - Required Rust API: send text/post, save draft, conversation events.
  - Swift implementation: multiline text input, 65536-character guard, send,
    silent send, persisted debounced drafts, optimistic clear/restore and
    visible send errors use Rust-owned conversation/message operations.
    `Send as Post` and Shift-Cmd-Return call the real post command and share the
    draft success/restore behavior. The AppKit-backed editor also preserves
    cursor-aware mention selection and IME composition behavior.
  - Validation: combined bridge/Xcode validation and signed-in text/silent/post
    shortcut verification remain pending.
- [x] Reply, quote, re-edit, selection, forward, recall and delete
  - Source: `_QuoteInputPreview`, `_SelectionBottomBar`,
    `_SelectionAction` — `flutter/lib/widgets/chat_view.dart`
  - Source: `MessageActionController.recallMessages` —
    `flutter/lib/controllers/message_action_controller.dart`
  - Source: `MessageContent` recall branch —
    `flutter/lib/widgets/message_content.dart`
  - Source: `_SendPage` — `flutter/lib/widgets/show_send_message_dialog.dart`
  - Source: `_ConversationSelector` —
    `flutter/lib/widgets/show_forward_conversation_selector.dart`
  - Required Rust API: message commands and forwarding.
  - Swift implementation: message context-menu reply drives a cancellable quote
    preview and sends `quote_message_id`; selection mode supports row toggling,
    formatted multi-message clipboard copy, cancel, and confirmed delete for me
    or delete for everyone. Pin/unpin, batch delete and batch recall execute
    through `SwiftAccountHandle` and refresh the active timeline, with visible
    mutation failures. `ForwardConversationSheet` provides a searchable
    destination picker and runs real single/batch forward or 2...99-message
    combined-transcript forwarding. A successful local text recall retains its
    content in presentation state for six minutes, capped at 100 entries; the
    recalled row exposes `Re-edit`, appends that content to the current draft,
    moves the cursor to the end and focuses the AppKit composer. The cache is
    never restored for remote or historical recalls and is cleared with the
    conversation lifecycle. Recall is available for completed messages within
    30 days: either message in a direct conversation, any group message for an
    owner, and regular-member messages for an admin. Rust repeats the permission
    checks and keeps the original local message until Blaze accepts the
    deterministic recall job; rejected recalls leave the original message
    unchanged, while acknowledged local cleanup resumes after restart without
    redelivery.
  - Validation: Rust recall policy/job/database tests, Flutter chat policy
    tests, Flutter analysis and the unsigned SwiftUI Debug build pass;
    signed-in recall, re-edit and mutation verification remain.
- [x] Mentions and keyboard selection
  - Source: `_MentionTextEditingController`, `_MentionPanelPortal`,
    `_MentionPanel`, `_MoveMentionIntent`, `_SelectMentionIntent` —
    `flutter/lib/widgets/chat_view.dart`
  - Source: `MentionController` —
    `flutter/lib/controllers/mention_controller.dart`
  - Required Rust API: group participants, friends and recent-message users.
  - Swift implementation: `MentionComposer`, `MentionComposerModel`,
    `MentionCandidatePanel`, `MentionTextEditor`; group empty queries load all
    participants except the current account, group keyword queries stay within
    the group, and bot queries use the existing friends plus recent-message-user
    search. Cursor-local `@` matching, stale-request cancellation, IME guards,
    four-row popup sizing, mouse selection, Up/Down/Tab, Control-N/Control-P,
    Enter-before-send, Escape dismissal, `@identityNumber ` insertion, mention
    highlighting and existing-draft mention-name resolution are implemented
    through the Rust account handle.
  - Validation: Rust bridge tests pass and the mention sources compile in the
    Xcode Debug target; the combined build and signed-in group/bot composer
    interaction remain pending.
- [x] Attachment picker, preview, caption, image edit and drag/drop
  - Source: `_pickAttachments`, `_showAttachments` —
    `flutter/lib/widgets/chat_view.dart`
  - Source: `_AttachmentPreviewDialog`, `_PreviewTab`, `_BigImageTile`,
    `_BigVideoTile`, `_PreviewFile`, `_ZipPage` —
    `flutter/lib/widgets/show_attachment_preview_dialog.dart`
  - Source: `_ImageEditorDialog` — `flutter/lib/widgets/image_editor.dart`
  - Source: `ChatDropOverlay`, `_ChatDragIndicator` —
    `flutter/lib/widgets/chat_drop_overlay.dart`
  - Required Rust API: send files/images/video, cancel, retry, progress.
  - Swift implementation: the composer file picker supports multiple files,
    media/file mode, image previews, removal/addition, single-image captions,
    normal or silent sequential sends, image dimensions, video dimensions and
    duration, reply attachment quoting, and visible send failures through the
    Rust attachment operation. Media images are resized to a 1,920-pixel
    maximum and encoded as JPEG or alpha-preserving PNG. The native editor
    supports crop presets, rotation, horizontal flip, colored drawing, undo
    and reset. Two or more files can be archived into one ZIP with an optional
    password, duplicate filenames are preserved with unique archive names, and
    video rows use the native player preview. Finder drops and clipboard files
    reuse this same preview/send flow.
  - Remaining: signed-in picker/editor/archive/drop/send verification.
    Timeline transfer cancel/retry/progress, native image/video/file viewers
    and file interaction are implemented under the core-message and preview
    nodes above.
- [x] Contact sharing
  - Source: `_sendContact`, `_SendActionTypeButton`,
    `_SendActionType.contact` — `flutter/lib/widgets/chat_view.dart`
  - Required Rust API: selectable contacts and contact-message send command.
  - Swift implementation: the composer attachment menu opens
    `ContactShareSheet`, which loads and filters selectable users, shows send
    progress and errors, preserves reply quoting, and sends through
    `SwiftAccountHandle.sendContact`.
- [x] Voice recording and audio playback
  - Source: `VoiceRecorderBarOverlayComposition`, `VoiceRecorderBar`,
    `_VoiceRecordingPreview` — `flutter/lib/widgets/chat_view.dart`
  - Source: `VoiceRecorderController`, `VoiceRecorderState`,
    `VoiceRecording`, `RustVoiceRecorderBackend` —
    `flutter/lib/controllers/voice_recorder_controller.dart`
  - Source: `AudioMessageWidget`, `_AudioStatusButton`,
    `AudioMessagePlaybackCoordinator` —
    `flutter/lib/widgets/message_audio.dart`
  - Rust implementation: `mixin_desktop_media` owns CPAL input/output,
    Ogg/Opus encode/decode, the 60-second recording limit, normalized waveform
    generation, playback position, seek, pitch-preserving WSOLA speed changes
    and playlist advancement.
    `mixin_desktop_api::MediaClient` exposes the shared process-scoped owner to
    FRB and UniFFI.
  - Swift implementation: `VoiceRecorderModel`, `VoiceRecorderBar`,
    `VoiceRecordingPreview`; requests real macOS microphone permission,
    delegates capture and encoding to `SwiftMediaHandle`, and supports stop,
    preview, discard, retry and quoted send through
    `SwiftAccountHandle.sendAudio`.
  - Swift implementation: `AudioMessageView`,
    `AudioPlaybackCoordinator`; observes the shared Rust player, renders
    waveform/duration and transfer states, supports play/pause, seek, 1x/2x
    speed, automatic next-message playback, download/cancel/retry, and marks
    newly played audio read.
  - Validation: Rust codec, resampling and waveform tests pass; Flutter
    controller/widget tests, Flutter macOS Debug build and the Xcode Debug
    build pass.
    Signed-in microphone permission, real recording/send/receive, audio output
    and interruption handling remain to be verified live.
- [x] Emoji, stickers, GIFs and sticker management
  - Source: `StickerButton` —
    `flutter/lib/widgets/sticker_page/sticker_button.dart`
  - Source: `StickerPage`, `_StickerAlbumPage`, `_StickerAlbumBar`,
    `_StickerAlbumBarItem` —
    `flutter/lib/widgets/sticker_page/sticker_page.dart`
  - Source: `StickerItem` —
    `flutter/lib/widgets/sticker_page/sticker_item.dart`
  - Source: `_StickerAsset` —
    `flutter/lib/widgets/message_content.dart`
  - Source: `StickerDetailPage`, `_StickerDetailLoading` —
    `flutter/lib/widgets/sticker_page/sticker_detail_page.dart`
  - Source: `EmojiPage`, `_EmojiPageBody`, `_EmojiGroupHeader`,
    `_AllEmojisPage`, `_EmojiItem` —
    `flutter/lib/widgets/sticker_page/emoji_page.dart`
  - Source: `GiphyPage`, `_GifGridView`, `_GifItem` —
    `flutter/lib/widgets/sticker_page/giphy_page.dart`
  - Source: `_StickerStorePage`, `_StoreAlbumItem`, `_StickerAlbumPage`,
    `_StickerAlbumManagePage` —
    `flutter/lib/widgets/sticker_page/sticker_store.dart`
  - Source: `_AddStickerDialog` —
    `flutter/lib/widgets/sticker_page/add_sticker_dialog.dart`
  - Source state owner: `StickerController` —
    `flutter/lib/controllers/sticker_controller.dart`
  - Required Rust API: sticker/album operations and remote image send.
  - Swift implementation: `StickerPanelView`, `StickerPanelModel`,
    `EmojiPickerView`, `GiphyPickerView`, `StickerStoreView`,
    `StickerAlbumManageView`, `StickerDetailView`; the composer popover loads
    cache-first recent/personal/album stickers, performs a throttled remote
    refresh, persists 35 recent emoji through Rust settings, inserts emoji,
    sends stickers, and exposes add/remove/detail actions.
  - Swift implementation: the store force-refreshes the real Mixin sticker
    catalog, previews albums, adds/removes albums, reorders the installed
    album list, and exposes loading, empty, error and retry states. GIF
    trending/search uses the source `MIXIN_GIPHY_KEY` contract, 51-item
    pagination and the real `sendRemoteImage` command; the GIF tab remains
    hidden when no key is configured.
  - Bridge: `SwiftStickerItem`, `SwiftStickerAlbumItem`,
    `SwiftStickerAlbumSection`, `SwiftStickerLibrary`,
    `SwiftStickerDetailItem`; coarse library/store queries plus sticker,
    album and remote-image commands are exposed through
    `mixin_desktop_api` and UniFFI.
  - [x] Animated JSON sticker playback
    - Source: `StickerItem`, `StickerGroupIcon` —
      `flutter/lib/widgets/sticker_page/sticker_item.dart`
    - Source: `_StickerAsset` —
      `flutter/lib/widgets/message_content.dart`
    - Source: `StickerButton` —
      `flutter/lib/widgets/sticker_page/sticker_page.dart`
    - Source: `StickerDetailPage` —
      `flutter/lib/widgets/sticker_page/sticker_detail_page.dart`
    - Source: `StickerStore`, `_StickerAlbumPage` —
      `flutter/lib/widgets/sticker_page/sticker_store.dart`
    - Swift implementation: `StickerLottieView`,
      `StickerLottieDataCache`, and the unified `StickerRemoteImage` render
      `assetType == json` assets in chat messages, the picker, album icons,
      store previews, and sticker detail. Playback loops while active,
      pauses/restores with the app and window lifecycle, caches downloaded JSON
      under `cache_lottie`, detects JSON through the parsed URL path, and exposes
      a visible retry state that also requests `refreshSticker`.
  - Validation: the sticker Rust bridge, all Swift sticker files, the Lottie
    renderer and final whole-app link pass in the clean Xcode Debug build.
    Signed-in sticker/store/GIPHY sends remain to be verified live.
  - Remaining: signed-in live verification for static/animated stickers,
    sticker store, and GIPHY sends.

## Conversation info, profile, and management

- [x] Chat inspector root and user/group profile
  - Source: destination pages selected through
    `ConversationInfoDestination` —
    `flutter/lib/pages/conversation_info_destination.dart`
  - Source: `MessageUserDialog`, `_ProfileBody`,
    `_UserProfileButtonBar` —
    `flutter/lib/widgets/show_message_user_dialog.dart`
  - Required Rust API: profile and conversation detail access.
  - Swift implementation: `ConversationInfoView`, `ConversationInfoModel`;
    cache-first conversation detail, background remote refresh, direct-user
    identity/biography, group announcement, avatar/name, copy-link, search,
    mute, pinned messages, shared content, shared apps and native inspector
    toggling are wired. `ProfileIdentityBadge` matches Flutter's active
    membership/verified/bot precedence. `MessageUserProfileView` implements
    `MessageUserDialog`'s cache-first refresh, biography, anonymous-user
    suppression, Option-click user-link copy, share/chat/information actions
    and relationship-aware add behavior. Direct chat info exposes confirmed
    Block, Unblock, Remove Contact/Remove Bot, Report and Block, share-contact
    and developer-profile actions through the real user facade, refreshing the
    authoritative profile after each mutation.
  - Validation: generated UniFFI bindings, Swift syntax parse and
    `cargo check -p mixin_desktop_swift` pass; signed-in relationship/profile
    inspection remains pending.
- [x] Participants, roles, invite/remove, shared groups
  - [x] `GroupParticipantsPage`, `_ParticipantTile`,
    `_showParticipantSelector` —
    `flutter/lib/pages/chat_side/group_participants_page.dart`
    - Swift implementation: `GroupParticipantsView`,
      `GroupParticipantsModel`, `AddGroupParticipantsSheet`; searchable full
      participant list, direct-message navigation, 1,024-member bounded add
      selector, OWNER-only admin promotion/demotion, OWNER/ADMIN-scoped remove
      actions and destructive removal confirmation use typed UniFFI commands.
  - [x] `_GroupInviteByLinkDialog`, `_ActionButton` —
    `flutter/lib/pages/chat_side/group_invite/group_invite_dialog.dart`
    - Swift implementation: `GroupInviteSheet`, `GroupInviteModel`; remote
      invite detail, native share, copy and confirmed link rotation are wired
      and visible only to OWNER/ADMIN participants.
  - [x] `GroupsInCommonPage` —
    `flutter/lib/pages/chat_side/groups_in_common_page.dart`
    - Swift implementation: `GroupsInCommonView`, `GroupsInCommonModel`;
      direct-user info loads authoritative common-group rows, renders
      loading/empty/error/content states, retries, and selects the group in the
      shared navigation owner.
    - Validation: generated bindings, Xcode build and signed-in selection smoke
      remain pending.
  - Required Rust API: group participants, user search,
    `updateParticipants`, `groupsInCommon`.
  - Validation: typed action bridge unit test and Xcode Debug build pass;
    signed-in live group management and shared groups remain.
- [x] Pinned messages, shared media/files/posts/apps, message search
  - [x] `PinnedMessagesPage`, `_PinnedMessage` —
    `flutter/lib/widgets/pinned_messages_page.dart`
    - Swift implementation: `ConversationPinnedMessagesView`,
      `ConversationPinnedMessagesModel`,
      `ConversationContentMessageRow`; message-change refresh, rich attachment
      rendering/actions, per-message unpin and confirmed unpin-all use the real
      pinned-message query and mutation.
  - [x] `SharedMediaPage`, `MediaPage`, `PostPage`, `FilePage`,
    `SharedMediaList`, `_MediaItem`, `_PostItem`, `_FileItem`,
    `ShareMediaItemMenuWrapper` —
    `flutter/lib/pages/chat_side/shared_media_page.dart`,
    `flutter/lib/pages/chat_side/share_media/media_page.dart`,
    `flutter/lib/pages/chat_side/share_media/post_page.dart`,
    `flutter/lib/pages/chat_side/share_media/file_page.dart`,
    `flutter/lib/pages/chat_side/share_media/shared_media_list.dart`
    - Swift implementation: `ConversationSharedContentView`,
      `ConversationSharedContentModel`, `SharedMediaGridItem`; native
      media/posts/files tabs use offset paging, day sections, message-change
      refresh, image/video previews and file attachment actions.
  - [x] `SharedAppsPage`, `_AppTile`, `_AppIcon`,
    `OverlappedAppIcons` —
    `flutter/lib/pages/chat_side/shared_apps_page.dart`
    - Swift implementation: `ConversationSharedAppsView`,
      `ConversationSharedAppsModel`; direct chats read the local shared-app
      cache first, refresh from the service, show app icons/descriptions and
      open the selected app in the native bot web window.
  - Source: `SearchMessagePage`, `_SearchMessageTile`, `_HighlightedText` —
    `flutter/lib/pages/chat_side/search_message_page.dart`
  - Required Rust API: pinned/search/shared-content queries and jumps.
  - Swift implementation: `ChatTimelineView`, `ChatTimelineModel`;
    conversation-scoped debounced message search, 60-row paging, sender/time
    result summaries and `messagesAround` timeline jumps are wired. The header
    button and native Cmd-F menu command share the same presentation state.
    `SwiftSharedAppItem` plus coarse `sharedMessages`, `localSharedApps` and
    `sharedApps` methods expose the existing `mixin_desktop_api` facade through
    UniFFI. Pinned/shared content rows issue a typed message jump through
    `HomeNavigationModel`; the selected chat consumes it with
    `messagesAround` and centers the target. Search supports participant/bot
    sender selection, Flutter-compatible Text/Post category chips, paged filter
    preservation and case-insensitive highlighted terms.
  - Validation: combined bridge/Xcode validation and signed-in live
    verification remain pending.
- [x] Conversation name, announcement, code, mute and disappearing messages
  - Source: conversation info destinations and `MuteDialog` —
    `flutter/lib/pages/conversation_info_destination.dart`,
    `flutter/lib/widgets/mute_dialog.dart`
  - [x] `ChatInfoPage._edit`, group name and announcement cells —
    `flutter/lib/pages/chat_side/chat_info_page.dart`
    - Swift implementation: `ConversationEditSheet`,
      `ConversationInfoModel.edit`; OWNER/ADMIN-only 40-character group name
      and 512-character announcement editing use real conversation mutations.
      Direct-user `Edit Name` reuses the same validated sheet and persists the
      1–40 character contact alias through `addContact(userId, fullName)`, with
      progress and operation-error presentation in the inspector.
  - Source: `_ConversationCodeDialog` —
    `flutter/lib/widgets/show_conversation_code_dialog.dart`
  - Required Rust API: edit conversation, rotate invite, mute and expiry.
  - Swift implementation: timed mute/unmute uses the real conversation command
    sheet; the inspector exposes Flutter's Off/30 seconds/10 minutes/2 hours/
    1 day/1 week disappearing-message presets plus custom seconds, minutes,
    hours, days or weeks, with owner/admin gating for groups and the real
    `setDisappearingMessages` operation. Group invite links support copy/share
    and confirmed rotation, while incoming group codes render avatars, name and
    participant count and execute the real join/open flow.
- [x] Clear/delete/exit group and destructive confirmations
  - Source: `ChatInfoPage`, `_ChatInfoPageState._confirm`,
    `_ChatInfoPageState._run`, destructive `CellGroup` —
    `flutter/lib/pages/chat_side/chat_info_page.dart`
  - Source: `HomeNavigationController.conversationDeleted` —
    `flutter/lib/controllers/home_navigation_controller.dart`
  - Source: `ChatSideNotifier.clear` —
    `flutter/lib/controllers/chat_side_notifier.dart`
  - Required Rust API: `clearConversation`, `deleteConversation`,
    `exitGroup`.
  - Swift implementation: `ConversationDestructiveActionsView`,
    `ConversationDestructiveActionsModel`; all actions require native
    destructive confirmation and expose progress/error states. `Clear Chat`
    deletes the local message history while preserving selection. Active group
    membership exposes the real remote `exitGroup` command; after participant
    loading shows the current account has exited, the same row becomes the
    local `deleteConversation` command. Successful exit/delete closes the
    inspector and clears the selected conversation through
    `HomeNavigationModel.conversationDeleted`.
  - Validation: `cargo test -p mixin_desktop_swift` and Xcode Debug build pass;
    signed-in live clear/exit/delete verification remains.

## Settings and account

- [x] Settings root, adaptive navigation and sign-out
  - Source: `SettingsPage`, `_SettingsHome`, `_UserProfile` —
    `flutter/lib/pages/settings_page.dart`
  - Required Rust API: profile plus account shutdown/sign-out.
  - Swift implementation: `SettingsView`; settings replaces the chat shell with
    a native two-column route, exposes every Flutter root destination, closes
    back to chats and confirms real account sign-out.
  - Validation: Swift bridge test and Xcode Debug build pass; compact layout
    and signed-in live navigation remain pending.
- [x] Edit profile and account
  - Source: `EditProfileSettingsPage`, `AccountSettingsPage`,
    `_ProfileField` — `flutter/lib/pages/settings_account_pages.dart`
  - Required Rust API: update profile/avatar, change number, delete account.
  - Swift implementation: `EditProfileSettingsView`, `AccountSettingsView`,
    `ChangeNumberSettingsView`, `DeleteAccountSettingsView` and
    `ProfileImageProcessor`; current profile fields, editable full
    name/biography, validation, save progress and recoverable errors use the
    account-scoped profile update command. The avatar picker performs a centered
    square crop, scales to 512 px, URL-safe Base64 encodes the JPEG and updates
    the authoritative Rust account profile through `avatar_base64`. Account is
    refreshed in the background while cached fields remain visible, and name /
    biography enforce the Flutter 40 / 140-character limits. Account is a
    first-class Settings destination with nested change-number and deletion
    warning/confirmation states. Phone verification and destructive
    deactivation remain visibly routed to Mixin Messenger until the desktop
    runtime owns wallet PIN/TIP and SMS verification.
  - Validation: avatar request serialization,
    `cargo test -p mixin_desktop_swift`, bridge-only clippy, formatting,
    `git diff --check`, isolated Swift type-checking of the image processor,
    final unified binding generation and the clean Xcode Debug build pass.
    Signed-in avatar upload remains pending.
- [x] Security, passcode and lock
  - Source: `SecuritySettingsPage`, `_SetPasscodeDialog` —
    `flutter/lib/pages/settings_account_pages.dart`
  - Source: `SecurityController` —
    `flutter/lib/controllers/security_controller.dart`
  - Source: `AuthGuard` — `flutter/lib/widgets/auth_guard.dart`
  - Required Rust API: durable security settings and authentication state.
  - Swift implementation: `SecuritySettingsView`, `SetPasscodeSheet`,
    `SecurityService`, `AuthGuard`, `UnlockOverlay` and
    `SixDigitPasscodeField`; the exact account-scoped
    `security.<accountId>.*` keys persist the confirmed six-digit passcode,
    Touch ID preference and Flutter-compatible 0/1/5/60/300-minute auto-lock
    duration in the Rust settings store. Saved passcodes lock during account
    restore, the scene lifecycle schedules and cancels inactivity locking,
    Shift-Cmd-L locks immediately, and the blocking blurred overlay supports
    passcode or macOS LocalAuthentication recovery.
  - Validation: the Security destination is reachable and the Xcode Debug build
    passes for the model, guard, settings screen and native Lock command.
    Signed-in lifecycle timing and live Touch ID verification remain pending.
- [x] Appearance
  - Source: `AppearanceSettingsPage`, `_ChatTextSizePreview`,
    `_PreviewMessage` — `flutter/lib/pages/settings_preference_pages.dart`
  - Source: `ChatView`, `_ChatMessageState`, `MessageName` —
    `flutter/lib/widgets/chat_view.dart`,
    `flutter/lib/widgets/message_name.dart`
  - Source asset: `MixinAssets.chatBackground` —
    `flutter/lib/constants/assets.dart`,
    `flutter/assets/images/chat_background.png`
  - Required Rust API: durable theme/text-size/message-name settings. The chat
    background is a bundled presentation asset and has no Flutter settings key.
  - Swift implementation: `AppearanceSettingsView`,
    `AppearanceChatPreview`, `SettingsPreferencesModel`,
    `ChatBackgroundView`, `ChatTimelineView.MessageRow`; the Rust settings
    store persists Flutter-compatible `brightness`, `messageShowAvatar`,
    `messageShowIdentityNumber` and `chatFontSizeDelta` values. Theme, message
    avatar visibility, sender `@identityNumber` visibility and chat text size
    apply immediately. The exact Flutter chat background PNG is bundled in the
    Swift asset catalog, uses the source light/dark template tint, and is
    applied to both the real timeline and Appearance preview. Theme selection
    is process-scoped and also covers launch, login and recovery UI.
  - Bridge: `SwiftMessageItem.senderIdentityNumber` exposes the existing
    `MessageListView.sender_identity_number` field through UniFFI.
  - Validation: bridge tests, bridge-only clippy, formatting, diff checks and
    the final clean Xcode Debug build pass. Signed-in visual verification of
    both appearance modes remains pending.
- [x] Notifications
  - Source: `NotificationSettingsPage` —
    `flutter/lib/pages/settings_preference_pages.dart`
  - Required Rust API: notification preferences and OS permission state.
  - Swift implementation: `NotificationSettingsView`; message-preview
    preference uses the Rust settings store, requests native alert/badge/sound
    authorization when undetermined, reads macOS authorization state and links
    to System Settings when disabled. `NotificationController` consumes this
    preference during real notification delivery.
- [x] Proxy and network
  - Source: `ProxySettingsPage`, `_AddProxyDialog`, `_ProxyCell` —
    `flutter/lib/pages/settings_preference_pages.dart`; `NetworkController` —
    `flutter/lib/controllers/network_controller.dart`
  - Required Rust API: proxy settings and active network status.
  - Swift implementation: `ProxySettingsView`, `ProxyEditorView`,
    `ProxySettingsModel`; the screen loads and persists the Rust-owned proxy
    settings, disables activation without a configured server, selects the
    first server by default, and supports enable, selection, add, edit and
    confirmed deletion with HTTP/SOCKS5, authentication, field validation,
    mutation progress and recoverable load/save errors. Deleting the effective
    selected server disables proxying, matching `NetworkController`.
    `SwiftDesktopHandle` exposes bridge-safe `SwiftProxySettings` and
    `SwiftProxyItem` records over `SettingsClient`.
  - Validation: the `.proxy` settings destination opens the native screen;
    bridge conversion unit tests and the macOS Xcode Debug build pass.
- [x] Storage policies and usage cleanup
  - Source: `StoragePage`, `StorageUsageListPage`,
    `StorageUsageDetailPage` —
    `flutter/lib/pages/settings_storage_about_pages.dart`
  - Required Rust API: typed auto-download settings, category/conversation
    usage, cleanup commands.
  - Swift implementation: `DataStorageSettingsView`,
    `StorageUsageListView`, `StorageUsageDetailView` and
    `StorageDirectoryMonitor` provide the three durable auto-download toggles,
    conversation-ranked usage, photo/video/audio/file selection and confirmed
    cleanup. Recursive FSEvents refresh the visible usage after media changes.
    `SwiftAccountHandle` exposes the screen-ready usage DTOs, media directory
    and cleanup command.
  - Validation: `cargo test -p mixin_desktop_swift` and the macOS Xcode Debug
    build pass.
- [x] Backup, restore, account deletion
  - Source: `BackupSettingsPage`, `AccountDeleteSettingsPage` —
    `flutter/lib/pages/settings_account_pages.dart`
  - Required Rust API: backup/restore/device transfer and account deletion.
  - Swift implementation: backup/restore is implemented by the account-wide
    device-transfer flow documented below. The Settings root `Chat Backup`
    entry directly invokes `DeviceTransferController.openSetup()`;
    `BackupSettingsView` also exposes the same action as a recoverable detail
    destination. `AccountSettingsView` exposes the account entry, deletion
    consequences, destructive confirmation and change-number alternative from
    the Flutter pages. Actual account deactivation and phone migration remain
    external because the current desktop runtime has no wallet PIN/TIP or SMS
    verification owner.
- [x] MCP and AI settings
  - Source: `McpSettingsPage` — `flutter/lib/pages/mcp_settings_page.dart`
  - Required Rust API: typed MCP settings and service lifecycle.
  - Swift implementation: `McpSettingsView`, `McpSettingsModel`; the page
    loads the real server settings and runtime status, enables or stops the
    localhost server, displays and copies the endpoint and masked bearer token,
    and controls draft-editing and circle-management permission scopes.
    `SwiftDesktopHandle` exposes bridge-safe MCP settings/status records over
    the existing Rust-owned service lifecycle.
- [x] About and logs
  - Source: `AboutPage`, `_AboutLink`, `SettingsLogPage` —
    `flutter/lib/pages/settings_storage_about_pages.dart`
  - Required Rust API: log directory/export where applicable.
  - Swift implementation: `AboutSettingsView`, `LogViewerSheet`; the page
    shows bundle version and native social/help/legal links, preserves the
    five-tap logo log viewer and seven-tap diagnostics reveal, reads the newest
    real Rust log with refresh/selectable text, and opens its Finder directory
    through the bridge-exposed log path.

## macOS and account-wide integrations

- [x] Notification delivery and click routing
  - Source: `HomeNotificationBridge` —
    `flutter/lib/widgets/home_notification_bridge.dart`
  - Required Rust API: notification stream, conversation/message resolution.
  - Swift implementation: `NotificationController`; the account notification
    stream delivers native `UNUserNotificationCenter` banners, applies the
    message-preview preference and Flutter category summaries, ignores stale
    and currently visible messages, removes recalled notifications, clears a
    conversation's notifications when selected, and activates/navigates the
    app when a banner is clicked. The notification payload's message ID is
    passed into the shared timeline locate request, so selection centers
    the exact clicked message after loading its surrounding window.
  - Remaining: signed-in live delivery, recall and exact-message selection
    verification.
- [x] Dock unseen badge
  - Source: `AppIconBadge` — `flutter/lib/widgets/app_icon_badge.dart`
  - Required Rust API: `unseenMessageCountChanges`.
  - Swift implementation: `DockBadgeController`; a cancellable account-scoped
    subscription drives the native `NSDockTile.badgeLabel` with the real
    unmuted unseen-message total and clears it on zero or teardown.
- [x] Device transfer
  - Source: `DeviceTransferHandlerWidget` and approval/progress dialogs —
    `flutter/lib/widgets/device_transfer_widget.dart`
  - Source: `_DeviceTransferNavigator`, `_RestorePage`, `_BackupPage` —
    `flutter/lib/widgets/device_transfer_dialog.dart`
  - Required Rust API: device-transfer event and command streams.
  - Swift implementation: `DeviceTransferController`,
    `DeviceTransferCoordinatorView`, `DeviceTransferSetupView` and
    `DeviceTransferProgressView`; `AccountSession` owns one account-wide,
    cancellable typed event subscription. Incoming pull/push requests show the
    matching approval and send confirm/reject commands. The manual Debug File
    menu flow supports restore/backup explanation and waiting pages, real
    pull/push commands, back/close cancellation, live progress and network
    speed, transfer cancellation, success/failure results and protocol-version
    mismatch recovery. The progress flow disables idle system sleep until it
    finishes or is cancelled.
  - Bridge: `SwiftDeviceTransferEvent`,
    `SwiftDeviceTransferCommand`, `SwiftConnectionFailedReason` and
    `SwiftDeviceTransferSubscription` expose the existing
    `mixin_desktop_api` stream and command contract without string/JSON
    lowering.
  - Validation: bridge unit tests and the macOS Xcode Debug build pass.
  - Remaining: signed-in two-device live validation for both directions,
    approval rejection, cancellation and version mismatch.
- [x] Clipboard, file open/save, drag/drop and media platform adapters
  - Source: `_PasteFilesAction` — `flutter/lib/widgets/chat_view.dart`
  - Source: `readClipboardFiles` —
    `flutter/lib/utils/system_clipboard.dart`
  - Source: attachment/media/dialog entry points under `flutter/lib/widgets/`
  - Required Rust API: attachment metadata and transfer commands; OS panels
    remain in Swift.
  - Swift implementation: native image clipboard copy, file open, Save As,
    Finder reveal, attachment picking and media playback are wired. The chat
    accepts native file URL drops, shows a targeted drop overlay and reuses the
    attachment preview/send flow. `MentionTextView` intercepts paste before
    normal text insertion, reads existing file URLs or PNG/JPEG/GIF/WebP/BMP
    image payloads from `NSPasteboard`, materializes image data into unique
    temporary files, and opens that same attachment preview/send flow. Plain
    text paste continues through AppKit unchanged.
  - Validation: unsigned Debug Xcode build passes; live Finder/image clipboard
    paste and signed-in attachment sending remain.
- [x] Web view title bar and external URL behavior
  - Source: `WebViewNavigationBar` —
    `flutter/lib/widgets/web_view_navigation_bar.dart`
  - Source: `openMessageAction`, `openBotWebViewWindow` —
    `flutter/lib/utils/web_view.dart`
  - Required Rust API: bot home URI lookup.
  - Swift implementation: `BotWebViewWindow` owns a retained 380x750 native
    `WKWebView` window with back, forward, reload and external-open controls,
    the `MixinContext` JavaScript contract, locale/appearance/currency context
    and Mixin user-agent suffix. App cards/button groups and the direct-bot
    header button are wired through `MessageActionHandler`; `input:` sends a
    real text message and Mixin schemes return to the application protocol
    handler.
  - Validation: combined bindings/Xcode build and a live bot-app action remain.

## Final validation gate

- [x] Every node above has source-flow evidence and no hidden/no-op action.
- [ ] Swift model and targeted UI tests cover loading/content/empty/error,
  cancellation, stale results, navigation, keyboard, menus and destructive
  confirmations.
- [x] UniFFI bindings regenerated and generated diff inspected.
- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace --all-targets`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo build -p mixin_desktop_swift`
- [x] Xcode Debug build and relevant test destinations.
- [ ] Live macOS smoke: startup, restore/login, Blaze/Signal, real messages,
  notifications, media, files, protocol links and sign-out.
- [x] `git diff --check` and final unrelated-change audit.
