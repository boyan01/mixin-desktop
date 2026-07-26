# Mixin macOS SwiftUI Working Agreement

## Product goal and truth sources

- `swiftui/` is the native macOS product target. Build its UI with SwiftUI and keep the reusable Rust core.
- Match the current in-repository `flutter/` product behavior and feature set. Exact pixels and widget implementation may differ; user-visible states, actions, navigation, keyboard behavior, context menus, drag and drop, lifecycle behavior, and error recovery must remain functionally equivalent.
- Re-read the current source implementation before porting behavior. Start at `main` and `runApp`, then follow the real descendant widget, provider, controller, database, and platform-integration paths for the feature.
- Keep in-repository `flutter/` read-only unless the user explicitly asks to change it. Preserve dirty files in both repositories.
- Maintain `swiftui/PARITY.md` as the living feature inventory once implementation starts. Each entry records the source entry path, required Rust API, Swift status, validation evidence, and a concrete unresolved question when blocked.
- The parity inventory records coverage; it never substitutes for tracing and implementing the real source flow.

## System architecture

Use this dependency direction:

```text
SwiftUI views
      |
      v
@Observable feature models
      |
      v
generated Swift UniFFI API
      |
      v
swiftui/rust (Swift-only lowering and DTO conversion)
      |
      v
mixin_desktop_api (platform-neutral public client API)
      |
      v
mixin_desktop_core + mixin-bot-sdk
```

- Rust owns protocol and API access, Blaze, Signal, sync, jobs, SQLite, credentials, durable settings, durable business state, and reusable business filtering.
- Swift owns macOS presentation, window and scene lifecycle, navigation, focus, keyboard commands, menus, notifications, drag and drop, media presentation, and view-local state.
- `DesktopRuntime` owns process-scoped Rust services. `AccountRuntime` remains authoritative for account-scoped business state.
- `mixin_desktop_api` is the shared platform-neutral facade. Add coarse clients, DTOs, commands, and event streams there when a capability should be available to more than one UI bridge.
- `swiftui/rust` depends on `mixin_desktop_api`, never directly on `mixin_desktop_core`. It contains UniFFI annotations, bridge-safe DTO conversion, typed errors, and subscription lowering only.
- Swift must not access SQLite, Mixin HTTP APIs, Blaze, Signal, credentials, or business repositories directly.
- SwiftData, Core Data, UserDefaults, and `@AppStorage` must not become a second profile, conversation, message, or settings database. Use them only for genuinely macOS-local presentation preferences such as window or column state when needed.
- Keep bridge calls coarse enough to avoid chatty FFI. Prefer one query that returns a screen-ready DTO window over many per-row or per-field calls.
- Keep DAOs, SQL rows, internal services, and Rust-specific path or synchronization types behind the public API.

## Swift source organization

Grow the Xcode source tree by feature. Use this target structure as the default:

```text
swiftui/
├── MixinMessenger/
│   ├── App/
│   │   ├── MixinMessengerApp.swift
│   │   ├── AppModel.swift
│   │   ├── AppPhase.swift
│   │   └── AppCommands.swift
│   ├── Session/
│   │   ├── AccountSession.swift
│   │   └── SessionLifecycle.swift
│   ├── Navigation/
│   │   ├── HomeNavigationModel.swift
│   │   ├── HomeRoute.swift
│   │   └── ChatInspectorRoute.swift
│   ├── Features/
│   │   ├── Authentication/
│   │   ├── Sidebar/
│   │   ├── Conversations/
│   │   ├── Chat/
│   │   ├── ChatInspector/
│   │   ├── Search/
│   │   ├── Attachments/
│   │   ├── Stickers/
│   │   ├── Profile/
│   │   ├── Settings/
│   │   ├── DeviceTransfer/
│   │   └── AI/
│   ├── DesignSystem/
│   │   ├── Components/
│   │   ├── Modifiers/
│   │   ├── Theme/
│   │   └── Assets/
│   ├── Platform/
│   │   ├── Notifications/
│   │   ├── DeepLinks/
│   │   ├── Pasteboard/
│   │   ├── Media/
│   │   └── Window/
│   ├── Bridge/
│   │   ├── AsyncSequence/
│   │   └── ErrorPresentation/
│   ├── Support/
│   │   ├── Logging/
│   │   ├── Localization/
│   │   └── Fixtures/
│   ├── Generated/
│   └── Assets.xcassets/
├── MixinMessengerTests/
├── MixinMessengerUITests/
├── generated/
└── rust/
```

- Keep generated Swift in `swiftui/MixinMessenger/Generated/` and generated C/module-map files in `swiftui/generated/`. Never hand-edit them.
- A feature owns its views, observable model, feature-local value types, and small components. Example: `Features/Chat/ChatView.swift`, `ChatModel.swift`, `MessageRowModel.swift`, and `Components/`.
- Start a feature with a flat folder. Add `Components/`, `Models/`, or `Tests/` subgroups only when the feature has enough files to justify them.
- Put a component in `DesignSystem/` after at least two independent features need the same semantic component. Do not move feature-specific presentation there for cosmetic reuse.
- Put AppKit wrappers and OS service adapters in `Platform/`. Keep `NSViewRepresentable`, delegates, notification callbacks, and window APIs out of feature models.
- Put hand-written UniFFI lifecycle helpers in `Bridge/` only when they add cancellation, `AsyncSequence`, error presentation, or testability. Do not add wrappers that only rename generated methods.
- Prefer one primary type per file. Name observable owners `*Model`, long-lived account capabilities `*Service`, navigation owners `*NavigationModel`, and bridge entry points `*Client` or `*Handle`.
- Avoid generic `Managers`, global `Utils`, and a project-wide `Models` dumping ground. Place helpers next to the feature that owns the behavior.
- Keep Swift feature value models presentation-oriented and disposable. Durable domain entities continue to live in Rust.

## State ownership and Observation

Use the native Observation framework. Do not add TCA, Redux, Riverpod-like containers, or another global state package without a demonstrated requirement that this model cannot satisfy.

| Lifetime | Owner | Responsibilities |
| --- | --- | --- |
| Process | `AppModel` | Open `DesktopClient`, restore or start login, switch `AppPhase`, create or destroy the account session |
| Account session | `AccountSession` | Own the account handle, profile snapshot, account-wide health, notification/device-transfer tasks, and coordinated shutdown |
| Window or scene | `HomeNavigationModel` | Sidebar selection, selected conversation ID, detail path, inspector route, presented sheet, and active command target |
| Feature | `ConversationListModel`, `ChatModel`, etc. | Load screen data, consume feature events, keep a bounded UI window, run user commands, expose explicit loading/content/empty/error state |
| View | `@State`, `@FocusState`, `@GestureState` | Hover, focus, disclosure, local selection, draft interaction, animation, and other ephemeral presentation state |

- Mark UI-facing observable reference types `@MainActor @Observable final class`.
- Create the process model once in `App` with `@State`. Create the window navigation model once at the window root. Create a feature model with `@State` at the feature boundary that owns its lifetime.
- Inject the signed-in `AccountSession` into the signed-in subtree with typed `@Environment(AccountSession.self)`. Do not make the session optional throughout unrelated views.
- Pass the narrow generated handle protocol or required dependency into a feature model initializer. Views may read environment values; models must receive dependencies explicitly.
- Views never call generated handles directly. User interaction goes through feature-model intent methods.
- Use `@Bindable` only where a control needs a binding to observable model state.
- Keep mutations inside model intent methods such as `selectConversation`, `send`, `retry`, or `setMuted`. Views render state and translate user interaction into intents.
- Represent mutually exclusive screen states with an enum or one coherent value. Avoid independent `isLoading`, `error`, and `items` fields that can describe contradictory UI.
- Keep derived state computed. Do not copy unread counts, selected conversations, profiles, or settings into multiple observable owners.
- Make drafts view-local while editing unless source behavior persists them. Persisted drafts go through the Rust conversation API.
- Use optimistic UI only for a clearly reversible interaction. Reconcile with Rust results or events and restore state on failure.

### App and session state machine

`AppModel` should expose one explicit root phase:

```text
launching
   ├── no saved account ──> signedOut
   ├── restored account ──> signedIn(AccountSession)
   └── recoverable failure ──> recovery

signedOut ── begin/wait login ──> signedIn(AccountSession)
signedIn ── sign out/shutdown ──> signedOut
```

- A successful restore or login creates exactly one `AccountSession` for the active account.
- The signed-in view tree exists only while its `AccountSession` exists.
- Sign-out cancels session tasks, invokes the Rust sign-out or shutdown operation, clears navigation state, and then changes the root phase.
- Database-open recovery, saved-login abort, and unauthorized state are explicit root states with user-visible actions.
- If saved-account switching is added for source parity, `AppModel` may own lightweight account descriptors. Only the active account owns an `AccountSession`.
- A window may own independent navigation and selection state. It must share the process/account runtime rather than opening a second authoritative runtime.

## Navigation and window composition

Use typed, value-based navigation. Route enums must be `Hashable`, contain stable identifiers, and contain no closures, views, or Rust handles.

```text
Wide window
┌──────────────┬─────────────────────┬──────────────────────────┬───────────────┐
│ Sidebar      │ Conversation list   │ Chat / settings detail   │ Inspector     │
│ profile      │ search + filters    │ timeline + composer      │ info/search/  │
│ circles      │ or settings list    │ or selected setting      │ pins/media    │
└──────────────┴─────────────────────┴──────────────────────────┴───────────────┘

Narrow window
┌──────────────────────────────────────────────────────────────────────────────┐
│ NavigationStack: sidebar → list → detail → contextual destination           │
└──────────────────────────────────────────────────────────────────────────────┘
```

- Build the main shell with a three-column `NavigationSplitView`: sidebar, content list, and detail. Use a native inspector or a nested typed route for chat information pages.
- Let SwiftUI manage column collapse and restoration first. Add custom width or compact-mode logic only for a verified interaction requirement.
- `HomeNavigationModel` owns `HomeSection`, selected conversation ID, detail path, inspector path, and sheet presentation. Feature models do not push global routes directly; they return an outcome or call a typed navigation intent.
- Use `NavigationStack` inside settings, chat inspector, and modal workflows that need their own path.
- Deep links, notification clicks, menu commands, and command-palette actions must resolve into the same typed navigation intents.
- Use `FocusedValues` and `Commands` for actions that target the active window or conversation. Keep business commands on the feature/session model.
- Use `List` for navigation collections where native selection and keyboard behavior fit. Use a bounded `ScrollView`/`LazyVStack` timeline for chat when variable-height messages, anchor jumps, and prepend/append behavior require it.
- Preserve selection, scroll anchors, search filters, and drafts across the same lifecycle boundaries as the source. Do not keep every heavy destination alive to achieve preservation.

## Async work, events, and cancellation

- Prefer Swift structured concurrency. Tie view-owned work to `.task(id:)` so SwiftUI cancellation follows view and identity changes.
- A model method called by `.task` should usually await its work instead of spawning an untracked nested task.
- Store a `Task` only when work intentionally spans multiple view calls. Cancel stored tasks in an explicit `stop()` and at the owning session or feature boundary.
- Avoid `Task.detached` for UI work. CPU-heavy parsing, crypto, database work, and sync belong in Rust.
- Check cancellation and request identity after suspension points when search text, selected conversation, account, or message anchor can change.
- Use generation/request tokens for operations whose underlying foreign call cannot be cancelled promptly. Ignore stale completion without overwriting newer state.
- Never block `MainActor` with synchronous FFI, file I/O, sleeps, semaphores, or database calls.

### Event flow

```text
Rust command or sync
       ↓
SQLite / authoritative runtime update
       ↓
typed account event or revision
       ↓
feature-owned subscription
       ↓
incremental re-query by stable IDs, or explicit full reload
       ↓
observable Swift presentation state
```

- Prefer events over polling.
- Expose Rust streams to UniFFI as typed subscription objects with `next() async throws -> Event?` and `cancel()` when direct stream generation is unavailable. Wrap them as `AsyncSequence` in Swift only when the wrapper adds structured cancellation and typed iteration.
- Each feature owns its relevant subscription. Do not route all account events through a global Swift event bus.
- `AccountSession` owns account-wide subscriptions only: notifications, application badge/unseen total, account health, device transfer, and other behavior that must continue without a feature view.
- Treat an event as an invalidation signal unless its contract contains a complete replacement value. Re-query affected IDs, coalesce bursts, and honor `reloadAll`.
- Every consumer must handle lag, closed streams, cancellation, retry, account shutdown, and scene activation.
- Render useful local/cache results immediately and refresh remote details in the background when the source follows cache-first behavior.
- Keep conversation and message UI windows bounded. Rust SQLite owns full history; Swift owns only the rows and anchors needed for the current viewport, prefetch, selection, and navigation.

## Bridge and generated code

- Stabilize the API shape in `mixin_desktop_api` before adding UniFFI annotations or regenerating bindings.
- Reuse `ClientResult<T>` and `ClientError` at the platform-neutral public boundary. Core internals may use `anyhow`; convert once at the public API boundary.
- Keep a single exhaustive conversion from `ClientError` to the UniFFI-exported Swift error. Do not create feature-specific string error bridges or scatter `map_err` adapters across methods.
- Preserve typed cases such as unauthorized, not found, cancelled, and invalid argument. Map internal failures to a loggable internal case and present a localized user message in Swift.
- Use UniFFI bridge-safe primitives, records, enums, byte arrays, and opaque objects. Expose filesystem paths as strings and timestamps with explicit units.
- Use account-scoped handles and access objects rather than exporting DAOs. Generated handle protocols are the first choice for Swift test doubles.
- Keep `uniffi` Tokio integration enabled and annotate exported async functions or impl blocks with `async_runtime = "tokio"`. Do not create a Tokio runtime per Swift handle or method.
- Generated bindings are committed. Regenerate Swift source, C header, and module map with the workflow in `swiftui/README.md` after every UniFFI-visible change.
- Inspect generated diffs for accidental API expansion, naming regressions, and formatting noise.
- A bridge helper is justified when it provides DTO conversion, error lowering, subscription cancellation, protocol adaptation, or batching. A rename-only wrapper is not justified.

## Feature implementation workflow

For each feature or vertical slice:

1. Trace the current `flutter-app` entry point, descendant widgets, providers/controllers, database/API calls, lifecycle hooks, menus, shortcuts, and platform callbacks.
2. Write down the observable contract: entry paths, loading/content/empty/error states, interactions, destructive confirmations, navigation outcomes, background updates, and recovery.
3. Locate the existing Rust implementation in `mixin_desktop_api` and `mixin_desktop_core`. Identify missing commands, DTOs, and events before building Swift state around gaps.
4. Add or stabilize the smallest coarse Rust API that owns the behavior. Add Rust contract tests first for new rules, queries, transitions, or persistence.
5. Implement the Swift feature end to end: model, view, navigation, keyboard/menu behavior, accessibility, cancellation, error presentation, and previews.
6. Add model tests and targeted UI tests. Run a live app smoke test when real startup, account state, Blaze, Signal, notifications, media, or file access is involved.
7. Compare the running Swift feature with the source behavior. Record any concrete unresolved parity question and leave unsupported actions visibly disabled until the Rust operation exists.

A feature is complete only when all source entry paths and user-visible actions work. A screen that renders while commands are silent no-ops, hidden, or permanently disabled is incomplete.

Suggested dependency order for broad implementation:

1. App bootstrap, saved-account restore, QR login, recovery, and sign-out.
2. Native window shell, sidebar, circles/categories, typed navigation, menus, and deep links.
3. Conversation list, unseen filter, search, selection, event refresh, and scroll restoration.
4. Chat read path, bounded message windows, anchors, unread state, quote/pin/search jumps, and attachment rendering.
5. Composer, send outcomes, reply, forward, recall/delete, selection, mentions, attachments, voice, stickers, and drag/drop.
6. Conversation info, participants, shared media/apps, pins, disappearing messages, groups in common, and circle management.
7. Profile, appearance, notifications, storage, security, proxy, backup, MCP, and AI settings/features.
8. Notification delivery/click routing, application badge, device transfer, audio player, protocol handler, and other macOS lifecycle integrations.

This order expresses dependencies only. Re-check the current source before every slice.

## SwiftUI and macOS implementation rules

- Prefer native SwiftUI controls, semantic colors, materials, accessibility, focus, and keyboard behavior. Functional parity does not require recreating Flutter's visual implementation.
- Use String Catalog localization. Do not ship user-facing strings from Rust unless the string is server content or a diagnostic detail.
- Keep design tokens semantic: background, sidebar, chat background, separator, primary text, secondary text, accent, destructive, unread, and muted.
- Previews use deterministic fixture DTOs and fake generated protocols. They never open the live Rust runtime or depend on a signed-in account.
- Isolate AppKit interop behind a small SwiftUI-facing type and document why SwiftUI alone cannot satisfy the behavior.
- App delegates and notification delegates translate OS callbacks into typed app/session intents. They do not own business state.
- Observe `scenePhase` at the scene/session boundary. Pause presentation-only timers and media work when inactive; keep Rust sync lifecycle decisions explicit.
- Avoid perpetual animations, timers, or display-link work when the state is static. Respect Reduce Motion and stop animation when inactive or off-screen.
- Log operation context, stable identifiers, error, and stack information where available. Do not log credentials, message plaintext, private keys, or attachment secrets.
- Confirm destructive operations in UI, then execute one Rust command and reflect its result. Do not pre-emptively mutate durable Swift state.

## Testing discipline

- Test observable contracts rather than incidental SwiftUI structure.
- Use Swift Testing or XCTest model tests under `MixinMessengerTests/`. Use `MixinMessengerUITests/` for a small set of high-value navigation, keyboard, menu, drag/drop, and accessibility flows.
- Model test names state the precondition, action, and expected result.
- Fakes conform to generated handle protocols where practical, fail on unexpected calls, record meaningful arguments, and use deterministic async streams.
- Cover initial loading, cached content, empty state, failure and retry, live event updates, cancellation, supersession, session shutdown, and stale-result guards where applicable.
- Test navigation models as pure typed state transitions.
- Test message window behavior with anchors, prepend/append, deduplication, scroll restoration, conversation switches, and late events.
- Rust unit tests cover pure rules, serialization compatibility, state transitions, and error classification.
- Rust database tests execute real SQLite queries and assert persisted rows, ordering, transaction rollback, and emitted events.
- Rust async tests use channels, barriers, paused Tokio time, or bounded timeouts. Do not use arbitrary sleeps.
- UI tests interact through accessibility identifiers, taps, keyboard, focus, menus, scrolling, and drag/drop. Assert user-visible results.
- Do not use screenshots, logs, or successful construction as the only oracle. Snapshot tests are optional and do not prove functional parity.
- Keep tests hermetic and order-independent. Close subscriptions, tasks, temporary files, notification handlers, windows, and global overrides in teardown.
- Every regression fix includes a test that fails for the original bug at the lowest owning layer.

## Validation

- Run the narrowest relevant tests while iterating, then validate every touched layer before handoff.
- Rust baseline from the repository root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

- Swift bridge and app baseline:

```sh
cargo build -p mixin_desktop_swift
xcodebuild \
  -project swiftui/MixinMessenger.xcodeproj \
  -scheme MixinMessenger \
  -configuration Debug \
  -derivedDataPath /tmp/mixin-messenger-swift-derived \
  CODE_SIGNING_ALLOWED=NO \
  build
```

- Run the relevant `xcodebuild test` destinations after test targets exist.
- For a UniFFI-visible change, regenerate all bindings using `swiftui/README.md`, then run Rust tests, Swift model tests, and the Xcode Debug build.
- For changes to the shared public API that also touch `flutter/`, run `flutter analyze`, relevant `flutter test` targets, and `flutter build macos --debug` when generated bindings, plugins, or native build behavior changed.
- Run a live app smoke test for startup, restore/login, Blaze/Signal, real event streams, notifications, media, file access, protocol links, and visual interaction changes.
- Do not claim visual or behavioral parity without running the relevant flow.
- Run `git diff --check`, inspect the final diff, and separate repository failures from local Xcode, CocoaPods, signing, or cache failures.

## Communication and commits

- Reply in Chinese and lead with a short conclusion. Keep code, comments, commit messages, PR titles, and technical documentation in English.
- Lead reviews with discrete, actionable findings backed by both source and destination paths.
- Show an ASCII UI sketch when proposing or handing off a UI/UX change.
- Keep changes minimal and logic explicit. Do not add entities, abstraction layers, or fallback behavior without a concrete need.
- Do not add `codex/` or `[codex]` prefixes to branch names, commits, or PR titles.
- Commit only when requested. Before committing, inspect the intended diff; afterwards verify `git status --short` and `git show --stat --oneline HEAD`.
