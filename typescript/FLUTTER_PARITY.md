# Flutter / TypeScript Parity Map

This document is a source-level parity audit for the desktop shell. Flutter is the reference tree for the comparison. It records what can be established from the named source files; it does not claim pixel parity without a running visual smoke test.

## Status semantics

- `verified`: the source contract and the relevant runtime/visual behavior have both been verified.
- `partial`: a TypeScript counterpart exists, but the source contract, layout, lifecycle, or visual behavior is not demonstrably 1:1.
- `pending`: the counterpart or the required evidence is incomplete.
- `blocked`: verification requires a live app/runtime or source outside this audit and that evidence is not currently available.

No item below is marked `verified` for visual parity. The current evidence is source inspection only.

## Current pass: 2026-08-13

Implemented source-level alignment for the first two layers:

- Added `typescript/src/shared/layout/desktopShellLayout.ts` mirroring Flutter's `DesktopShellLayout.resolve` constants and drawer/compact/full-rail decisions.
- Added `typescript/src/app/components/DesktopShell.tsx`; `App` now mounts the shell through this component instead of owning the full grid directly.
- Added a narrow-window navigation drawer, compact rail sizing, route-mode mounting, and mobile navigation triggers.
- Added an `AppLifecycle` root wrapper for visibility focus restoration and the desktop `Cmd/Ctrl-W` hide-window shortcut.
- Reused Flutter sidebar SVG assets through CSS masks, matched the UUID bit-hash circle colors, and made collapse controls follow auto-collapse semantics.
- Added the Flutter chat background asset/filter, conversation empty asset, `Expanded`-equivalent list sizing, nested selected row surface, and untruncated unseen counts.
- Routed message search through the shell side destination, preserving root-side close, nested back, and narrow route result behavior.
- Added the nested chat-side breakpoint from Flutter's `_ChatWithSide`: the conversation list remains mounted while an info destination replaces chat until the remaining chat area reaches the 620px side-route threshold.
- Moved Settings into the shell's main content area, matched Flutter's 300px internal settings navigation and 620px breakpoint, and added narrow settings list/detail back navigation.

These changes are source-level and pass lint, typecheck, and production build validation. No running-window visual smoke test has been performed, so the affected items remain `partial`.

## Widget tree

```text
flutter/lib/main.dart::main
└── MixinDesktopApp
    ├── _AppLifecycleScope
    ├── _WindowShortcuts / GlobalMoveWindow
    └── MaterialApp
        └── _AppBody
            ├── _StartupPage / LoginPage / failure pages
            └── _SignedInBody
                ├── NetworkController / SecurityController / CoreHttpScope
                ├── AuthGuard
                └── HomePage
                    ├── Provider tree / Portal
                    └── _HomeBody
                        ├── HomeSidebar
                        ├── ConversationListPane
                        ├── _ChatWithSide
                        │   ├── ChatView
                        │   └── ChatSideRouter
                        ├── _ProfileSettingsPage / SettingsPage
                        └── _ProfileSetupGate / _SetupNameOverlay

typescript/src/main.tsx::createRoot
└── AppProviders
    └── QueryClientProvider / Tooltip.Provider
    ├── AppLifecycle
    │   └── App
    │       ├── LaunchState / Login
    │       └── DesktopShell
    │           ├── Sidebar or mobile drawer
    │           ├── ConversationList
    │           ├── Chat
    │           ├── ConversationInfo / Settings
    │           └── MessageSearchPanel, AuthGuard, dialogs, health overlays
    └── DocumentSelectionToolbar
```

## 1. Process bootstrap and root providers

### Flutter top widget: `main` → `MixinDesktopApp`

- **Flutter source:** `flutter/lib/main.dart:27-119`, `flutter/lib/app.dart:35-85`
- **TypeScript counterpart:** `typescript/src/main.tsx`, `typescript/src/app/AppProviders.tsx`, `typescript/src/app/AppLifecycle.tsx`; application composition lives in `typescript/src/app/App.tsx`.
- **Size/layout:** Flutter initializes a desktop window with a minimum size of `384 x 480` and a Windows default of `1280 x 750`; `MaterialApp` owns the full window. TypeScript sizes `html`, `body`, and `#root` to the viewport and lets `.desktop-shell` fill it; the desktop grid starts at `176px 300px minmax(320px, 1fr)` in `typescript/src/app/styles/shell.css`.
- **Color/theme:** Flutter installs light/dark `buildMixinTheme` and a persisted `themeMode`; TypeScript installs CSS variables in `typescript/src/app/styles/foundation.css`. The tokens are similar in role but are not proven equal in value or rendering.
- **Interaction/lifecycle:** Flutter initializes Rust, logging, notifications, the desktop window, protocol registration, settings, desktop handles, and `OverlaySupport` before `runApp`. TypeScript creates a React root through `AppProviders`; `AppLifecycle` owns focus restoration and the desktop hide-window shortcut, while `App` starts bootstrap queries and account effects.
- **Status:** `partial`
- **Evidence / next action:** The provider and startup responsibilities are still different in `flutter/lib/main.dart:27-87` versus `typescript/src/main.tsx`, `AppProviders.tsx`, and `AppLifecycle.tsx`. Run both desktop targets through cold start, suspend/resume, protocol launch, and window resize before assigning any visual or lifecycle parity as verified.

## 2. Application shell, authentication, and failure states

### Flutter: `_AppBody` → `_SignedInBody` → `AuthGuard`

- **Flutter source:** `flutter/lib/app.dart:207-257`, `flutter/lib/app.dart:476-535`
- **TypeScript counterpart:** `typescript/src/app/App.tsx`, `typescript/src/features/auth/components/Login.tsx`, `typescript/src/features/auth/components/AuthGuard.tsx`, `typescript/src/app/components/DesktopShell.tsx`
- **Size/layout:** Flutter selects startup, login, signed-in, and failure pages as mutually exclusive bodies inside `MaterialApp`; signed-in content is wrapped by account/network/security providers. TypeScript returns `LaunchState`, `Login`, or the grid shell from `App`, while `AuthGuard` is mounted as an overlay-like child inside the signed-in shell. The nesting and loading geometry are therefore only approximately corresponding.
- **Color/theme:** Flutter failure and landing pages use `LandingScaffold` and `context.mixinTheme`; TypeScript uses the global CSS variables and login/launch classes. Exact spacing, typography, dialog surfaces, and failure-page colors remain unmeasured.
- **Interaction/lifecycle:** Flutter `AppController.stage` drives transitions and supports login failure/database recovery callbacks. `_SignedInBody` initializes and disposes `NetworkController` and `SecurityController`, then waits for security initialization. TypeScript uses a React Query bootstrap, `useSecurity`, `useNotifications`, and `AuthGuard`; retry is query refetch, and signed-out state is written back into the query cache.
- **Status:** `partial`
- **Evidence / next action:** The state transitions are present in both implementations, but they use different owners and failure surfaces. Exercise startup, invalid database, login failure, sign-out, lock, and retry states on both targets and compare screenshots at the same window size.

## 3. Signed-in home composition

### Flutter: `HomePage` → `_HomeBody`

- **Flutter source:** `flutter/lib/pages/home_page.dart:103-378`
- **TypeScript counterpart:** `typescript/src/app/components/DesktopShell.tsx`, `typescript/src/shared/layout/desktopShellLayout.ts`, composed from `typescript/src/app/App.tsx`
- **Size/layout:** Flutter creates the filter, list, search, and viewport providers, then resolves `drawer`, `compactRail`, or `fullRail` from available width. The desktop shell uses a `64-176px` navigation rail, a fixed `300px` conversation pane, and the remaining chat area; narrow layouts route into a back-navigable chat. TypeScript now uses the same numeric layout rules in `resolveDesktopShellLayout`, mounts either the list or chat in route mode, and renders a `176px` navigation drawer at the drawer threshold. CSS keeps the same `176px 300px minmax(320px, 1fr)` wide geometry.
- **Color/theme:** Flutter paints the shell with `context.mixinTheme.primary` and the empty chat with `chatBackground`; TypeScript uses `--primary`, `--chat-background`, and `--background`. Both have light/dark roles, but the Flutter overlay alpha and TypeScript `color-mix` behavior require runtime comparison.
- **Interaction/lifecycle:** Flutter owns drafts, selection, message-location requests, recent-conversation recording, notification dismissal, and `HomeNavigationController` disposal. It also wraps the shell in shortcuts, macOS menu, profile setup, transfer, health, protocol, and notification bridges. TypeScript keeps selection/settings/dialog state in `useDesktopUi` and React state, with effects for account events, protocol URLs, keyboard shortcuts, menu actions, and recent conversations.
- **Status:** `partial`
- **Evidence / next action:** Flutter's source is `home_page.dart:44-100` and `home_page.dart:278-340`; TypeScript's pure layout rule is `shared/layout/desktopShellLayout.ts`. Validate actual window widths `384px`, `495px`, `620px`, and `796px`, drawer focus/dismissal, back navigation, draft preservation, and side-panel transitions before changing the status.

## 4. Primary navigation sidebar

### Flutter: `HomeSidebar`

- **Flutter source:** `flutter/lib/widgets/home_sidebar.dart:27-180`, `flutter/lib/widgets/home_sidebar.dart:407-768`
- **TypeScript counterpart:** `typescript/src/features/conversations/components/Sidebar.tsx`; shared primitives live under `typescript/src/shared/ui/`.
- **Size/layout:** Flutter receives `collapsed`, `showCollapse`, and selection callbacks from `HomePage`; the parent supplies `64px` compact width or `176px` full width and can place the same sidebar in a drawer. TypeScript uses `.sidebar` with `12px` horizontal padding, a `64px` collapsed width, a `64px` title-bar drag region, and a `56px` profile row in `app/styles/sidebar.css`. The profile, five category items, circles, badges, and collapse control are present in both trees.
- **Color/theme:** Flutter uses a divider and a `1%` white light overlay / `3%` black dark overlay at `home_sidebar.dart:64-70`, plus `context.mixinTheme` colors. TypeScript uses explicit light `rgba(255,255,255,.01)` and dark `rgba(0,0,0,.03)` backgrounds, Flutter SVG masks, `--sidebar-selected`, `--divider`, `--accent`, and `--secondary`. Icon sizes, fonts, tooltip placement, and overlay blending are not visually verified.
- **Interaction/lifecycle:** Flutter streams unseen counts and circles from `AccountHandle`, keeps local circle order with hooks, supports category/profile selection, collapse, drag reorder, context menus, rename, manage-conversations, and delete. TypeScript queries circles, derives unseen counts from bootstrap conversations, uses local ordered-circle state, supports drag reorder, context menus, dialogs, and a Zustand collapse toggle. Error rollback and stream disposal semantics are not equivalent by inspection.
- **Status:** `partial`
- **Evidence / next action:** TypeScript now accepts Flutter-equivalent `collapsed`, `showCollapseControl`, and drawer close ownership, and `circleColor` in `Sidebar.tsx` uses the UUID bit hash. Compare Flutter stream ownership and `super_context_menu` usage with TypeScript Query/Radix behavior. Run light/dark screenshots in full, compact, and drawer modes, then test circle reorder failure rollback and exact badge width.

## 5. Conversation list pane

### Flutter: `ConversationListPane` → `ConversationListView`

- **Flutter source:** `flutter/lib/pages/conversation_list_pane.dart:1-19`; the pane delegates its content to `flutter/lib/widgets/conversation_list_view.dart`.
- **TypeScript counterpart:** `typescript/src/features/conversations/components/ConversationList.tsx` and `CreateDialogs.tsx`, with primitives under `typescript/src/shared/ui/`.
- **Size/layout:** Flutter gives the pane a fixed `300px` slot in the wide home branch and paints a right divider; `ConversationListPane` itself is a `RepaintBoundary` around `ConversationListView`. TypeScript uses `.conversation-pane` as a flex column with a `64px` toolbar, an `Expanded`-equivalent `flex: 1` list, and a `51px` audio bar in `app/styles/conversations.css`. Rows remain `78px`, with an outer `8px` horizontal inset and a nested selected surface matching Flutter's row structure. Empty state uses Flutter's `58x78` asset.
- **Color/theme:** Flutter uses `mixinTheme.primary` and `mixinTheme.divider`; TypeScript uses `--primary`, `--background`, `--list-selected`, `--divider`, `--secondary`, and `--accent`. The TypeScript list has explicit selected, unread, muted, network-warning, empty, and audio-player colors.
- **Interaction/lifecycle:** Flutter providers own filtering, list loading, search, viewport paging, and account streams above this boundary. TypeScript derives category, circle, unread-only, query, and sorting with `useMemo`, listens for menu actions, opens create/mute/delete dialogs, and invalidates React Query data after mutations. The two list boundaries have different ownership and paging mechanisms.
- **Status:** `partial`
- **Evidence / next action:** TypeScript now enters its search tree for any non-empty query and keeps full unseen counts, matching the source-level behavior in `conversation_list_view.dart:132-143` and `conversation_list_view.dart:868-951`. Inspect and compare controller paging with the TypeScript list at identical data sets. Verify search focus/Escape, pinned ordering, unread filtering, network status, pagination, selection, and audio bar placement.

## 6. Selected conversation and side destinations

### Flutter: `_ChatWithSide` → `ChatView` / `ChatSideRouter`

- **Flutter source:** `flutter/lib/pages/home_page.dart:506-601`
- **TypeScript counterpart:** `typescript/src/features/chat/components/Chat.tsx`; `ConversationInfo` and `MessageSearchPanel` live under `typescript/src/features/conversations/components/` and are composed by `typescript/src/app/App.tsx`.
- **Size/layout:** Flutter computes a second breakpoint at `kResponsiveNavigationMinWidth + kChatSidePageWidth`; wide mode shows chat beside a `300px` side destination, while narrow mode uses `MaterialPage` navigation with the chat as a leading page. TypeScript keeps the conversation list mounted, routes the chat-side destination below the nested 620px threshold, and adds the `300px` info column only when the remaining chat area is wide enough (`shared/layout/desktopShellLayout.ts`, `app/styles/responsive.css`). The destination page still uses local React state rather than a separate Navigator page stack.
- **Color/theme:** Flutter inserts a one-pixel divider from `mixinTheme.divider`; TypeScript uses the grid's pane borders and `--divider`. Side-page surfaces and transitions have no source-level proof of matching.
- **Interaction/lifecycle:** Flutter keys each chat page by conversation ID and drives destinations through `ChatSideNotifier`. TypeScript keeps the destination stack inside `ConversationInfo`, models search origin in `useDesktopUi`, and mounts search through `DesktopShell`. Pinned and other destination pages still use local React state rather than a separate Navigator page stack.
- **Status:** `partial`
- **Evidence / next action:** Test switching conversations while info/search/pinned destinations are open, narrow back navigation, message-location requests, and deletion. Capture route transition and panel-width screenshots; no visual smoke has run yet.

## 7. Chat view, message timeline, and composer

### Flutter: `ChatView`

- **Flutter source:** `flutter/lib/widgets/chat_view.dart:45-220`, `flutter/lib/widgets/chat_view.dart:587-868`
- **TypeScript counterpart:** `typescript/src/features/chat/components/Chat.tsx`, `ChatTimeline.tsx`, `ChatComposer.tsx`, `MessageRow.tsx`, `MessageContent.tsx`, and `ChatOverlays.tsx`; styles are in `typescript/src/app/styles/chat.css` and `composer.css`.
- **Size/layout:** Flutter composes a `64px`-class `ChatHeader`, an expanded decorated message region, a `Stack`-based message list, and the input/selection area. It applies the chat background asset with a light `3%` black or dark `2%` white filter. TypeScript uses `.chat-pane`, `.chat-header` at `64px`, a scrollable message region, `.message-row` with `max-width: min(720px, 88%)`, and a composer with `min-height: 56px` and `8px 16px` padding in `app/styles/chat.css` and `composer.css`. These measurements are similar in intent, not yet confirmed as visual parity.
- **Color/theme:** Flutter uses `context.theme.primary`, `chatBackground`, `divider`, and the bundled chat background asset. TypeScript now imports the same `chat_background.png` and applies the light `3%` black / dark `2%` white layer on `.chat-drop-target`, while mapping the remaining tokens to `--primary`, `--chat-background`, `--divider`, `--bubble`, `--mine-bubble`, and `--encrypt`. Typography, image fit, and alpha blending still need screenshot comparison.
- **Interaction/lifecycle:** Flutter owns `MessageController`, `MessageActionController`, `ChatScrollCoordinator`, input/focus controllers, voice recorder, sticker controller, highlight animation, selection, quote/re-edit, attachment preview, contact/sticker/post/text send, forward/combine-forward, bot launch, pinned/search/info navigation, and disposal. TypeScript's `Chat` uses React state/query hooks for messages, participants, mentions, selection, quote, attachments, recording, media preview, pinned messages, transcript/location/special message dialogs, and send mutations. Both implement message locating/highlighting and rich content, but async cancellation, controller replacement guards, and disposal are not equivalent by source inspection.
- **Status:** `partial`
- **Evidence / next action:** Run Flutter and TypeScript message flows side by side. Verify keyboard/focus, scrolling and restore direction, quote jump, selection toolbar, attachment drop, voice recording, media preview, and send failure states.

## 8. Profile settings page

### Flutter: `_ProfileSettingsPage` → `SettingsPage`

- **Flutter source:** `flutter/lib/pages/home_page.dart:380-408`
- **TypeScript counterpart:** `typescript/src/app/components/DesktopShell.tsx`; `typescript/src/features/settings/components/Settings.tsx` and `SettingsCells.tsx`.
- **Size/layout:** Flutter replaces the main content area with `SettingsPage` while the sidebar remains in the home shell. TypeScript renders `.settings-page` as the shell's main grid child, keeps the sidebar visible, uses a 300px settings navigation column in wide mode, and switches to a list/detail route at 620px (`features/settings/components/Settings.tsx`, `app/styles/settings.css`, `app/styles/responsive.css`).
- **Color/theme:** Flutter receives profile/account state and uses the Mixin theme; TypeScript uses `--background`, `--primary`, `--popup`, `--divider`, and settings card styles. No exact typography or panel spacing equivalence is established.
- **Interaction/lifecycle:** Flutter streams profile changes, updates and refreshes through `AccountHandle`, signs out through `AppController`, and closes by returning to the chat category. TypeScript uses controlled tab state, settings queries/mutations, profile update cache writes, and a close callback; sign-out clears the bootstrap query. The TypeScript settings surface contains additional panels such as downloads, proxy, MCP, appearance, and security whose Flutter counterparts are not represented in the requested source set.
- **Status:** `partial`
- **Evidence / next action:** Compare every settings tab and sign-out/profile-refresh path against `SettingsPage`, then verify narrow back behavior, persistence, loading, and error states in both themes. No visual smoke has run yet.

## 9. Profile setup gate

### Flutter: `_ProfileSetupGate` → `_SetupNameOverlay`

- **Flutter source:** `flutter/lib/pages/home_page.dart:410-504`
- **TypeScript counterpart:** `typescript/src/features/account/components/AccountOverlays.tsx`; CSS is in `typescript/src/app/styles/startup.css`.
- **Size/layout:** Flutter keeps the home content mounted and places a full-screen `Stack` overlay with a centered `AlertDialogLayout`; TypeScript uses a fixed full-screen grid overlay with a centered `360px` maximum-width dialog. Both require a non-empty name and cap it at 40 characters.
- **Color/theme:** Flutter uses `mixinTheme.background` plus the shared dialog/button primitives; TypeScript uses `--background`, `--popup`, and `primary-button` styling. Dialog radius, shadow, text field, and button metrics are not confirmed equal.
- **Interaction/lifecycle:** Flutter owns a `TextEditingController`, listener, disposal, loading/success/failure toasts, and account profile update. TypeScript owns name/error/saving state with `useState`, submits through `desktopApi.updateProfile`, and renders inline errors. Both keep the underlying home mounted, but error and success feedback differ.
- **Status:** `partial`
- **Evidence / next action:** Compare initial focus, IME behavior, validation, update failure, success feedback, and whether the overlay blocks shell interaction across platforms.

## 10. Cross-cutting overlays and platform integrations

### Flutter wrappers under `_HomeBody`

- **Flutter source:** `flutter/lib/pages/home_page.dart:350-376`; related wrappers are `AccountHealthOverlays`, `AppProtocolHandler`, `HomeNotificationBridge`, `DeviceTransferHandlerWidget`, and `AppIconBadge`.
- **TypeScript counterpart:** `typescript/src/app/App.tsx`, `typescript/src/features/protocol/hooks/useProtocolUrls.ts`, `typescript/src/features/protocol/components/ProtocolDialogs.tsx`, `AccountOverlays.tsx`, `DeviceTransferDialog.tsx`, `ToastHost`, and `DocumentSelectionToolbar`.
- **Size/layout:** Flutter composes wrappers around the home tree and uses modal/overlay widgets; TypeScript renders fixed overlays and dialogs as siblings inside `.desktop-shell` or at the root. Z-index and coverage are CSS-specific (`app/styles/dialogs.css`, `protocol-dialogs.css`, and `startup.css`) and are not equivalent by source structure.
- **Color/theme:** Flutter uses theme extensions and shared overlay/dialog primitives; TypeScript uses `--popup`, `--background`, `--red`, `--accent`, and fixed overlay colors. Protocol QR, account-health, toast, and selection surfaces need runtime inspection.
- **Interaction/lifecycle:** Flutter handles platform protocol registration at bootstrap, app lifecycle/focus, notifications, system tray, device transfer, account health, and window menus through dedicated widgets/controllers. TypeScript handles pending/live protocol URLs, macOS menu updates, notifications, account health, device transfer, and selection toolbar through React effects/components. Cleanup is explicit in some TypeScript effects and Flutter disposers, but the platform event contracts differ.
- **Status:** `partial`
- **Evidence / next action:** Verify overlay precedence and cleanup after sign-out, lock, protocol navigation, notification click, device transfer, and app suspend/resume. Add a live smoke-test record before using `verified`.

## 11. TypeScript component-level counterparts

The following `typescript/src/shared/ui/` and `typescript/src/features/` components participate in the major tree above. They are mapped by responsibility rather than treated as proof that their Flutter visuals match.

| TypeScript component(s) | Flutter responsibility suggested by the audited tree | Current status | Evidence / next action |
| --- | --- | --- | --- |
| `Avatar.tsx`, `Badges.tsx` | Sidebar, conversation rows, chat header/message rows, dialogs | `partial` | Used throughout `Sidebar`, `ConversationList`, and `Chat`. Compare with Flutter's `avatar_view.dart` and message/avatar widgets at fixed sizes. |
| `IconButton.tsx`, `SearchTextField.tsx`, `InteractiveDecoratedBox.tsx`, `HoverOverlay.tsx` | Flutter shared controls, search field, hover/portal behavior | `partial` | CSS dimensions and hover states live mainly in `app/styles/shared-controls.css` and `app/styles/conversations.css`; keyboard, tooltip, focus-ring, and pointer behavior need live verification. |
| `MixinDialog.tsx`, `CustomPopupMenu.tsx`, `CreateDialogs.tsx` | Flutter dialogs, context menus, conversation selectors, circle actions | `partial` | Directly used by `Sidebar` and `ConversationList`; compare modal placement, focus trapping, dismissal, and loading/error states. |
| `RichText.tsx`, `MixinImage.tsx`, `LottieSticker.tsx` | Flutter message text, media, sticker and attachment content | `partial` | `MessageContent.tsx` routes many message categories through these components. Compare text metrics, image fit/fallback, animation lifecycle, and accessibility. |
| `AdaptiveSelectionToolbar.tsx` | Flutter selection toolbar and message selection actions | `partial` | Mounted by `AppProviders`; chat selection controls are owned by `Chat`. Verify selection geometry and action enablement. |
| `Settings.tsx`, `SettingsCells.tsx` | Flutter `SettingsPage` and settings cells | `partial` | Wide shell placement, 300px navigation, and 620px list/detail behavior are source-aligned; tab content, notification lifecycle, typography, and runtime visuals still need comparison. |
| `Login.tsx`, `AuthGuard.tsx`, `DeviceTransferDialog.tsx`, `UserProfileDialog.tsx`, `SnapshotDetailDialog.tsx`, `StickerDetailDialog.tsx` | Flutter login/auth guard and account/message destination dialogs | `pending` | Counterparts exist, but their Flutter sources are not among the requested audit files. Extend the evidence set and compare runtime flows. |

## 12. Cross-cutting parity gaps and next actions

1. **Responsive shell:** validate Flutter's drawer/compact/full-rail modes and route-based narrow chat against the TypeScript responsive shell at every breakpoint.
2. **State ownership:** document the mapping between Provider/controllers/Rust streams and Zustand/React Query/API effects, especially list paging, drafts, message windows, and disposal.
3. **Theme tokens:** produce a token table from `flutter/lib/theme.dart` and `typescript/src/app/styles/foundation.css`; compare rendered colors, alpha overlays, text sizes, borders, radii, and shadows in light and dark modes.
4. **Chat behavior:** run a deterministic matrix for selection, quote jump, message restore direction, attachments, voice, stickers, media preview, pinned messages, and send failures.
5. **Platform lifecycle:** verify startup, protocol URLs, notifications, menu actions, lock, suspend/resume, sign-out, and window resize with clean teardown.
6. **Visual evidence:** capture same-size screenshots for full shell, compact shell, drawer, empty chat, selected chat, settings, profile setup, dialogs, and dark theme. Only then promote an item to `verified`.
