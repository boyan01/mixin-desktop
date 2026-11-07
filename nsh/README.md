# Mixin Messenger · Native Shell

The native Rust UI depends on `mixin_desktop_api`. Protocol, credentials, SQLite,
Signal, Blaze, sync, jobs, secure screen-lock credentials, media preparation and
durable settings remain in the shared Rust business layer. NSH owns presentation,
navigation, disposable editing state and native platform adapters.

The migration from the Flutter desktop app (`/Users/yangbin/workspace/mixin/flutter-app`)
is in progress. See [docs/parity.md](../docs/parity.md) for per-feature status and
[ROADMAP.md](ROADMAP.md) for remaining work.

## Run

Use NSH 0.4.0, Rust and the platform's native SDK. From the repository root:

```sh
cargo run -p mixin_desktop_ui
```

The native executable defaults to `~/.mixin/mixin-nsh`. Use
`MIXIN_DESKTOP_DATA_DIR` to select an isolated test directory. Existing Flutter
account data is not imported (deferred). Do not open the same data directory with
another version while a runtime check is running.

The NSH CLI bypasses the native executable entry point. Prefer `cargo run` when
validating startup, process ownership, native component registration and shutdown.
macOS is the first validation target. Windows and Linux are unverified; see the roadmap.

GIF discovery requires the distributor's Giphy client configuration, as the
Flutter reference does. Set `MIXIN_GIPHY_API_KEY` in the build environment to
embed that configuration in a packaged app; a runtime environment value can
override it. An unconfigured build reports that GIF discovery is unavailable.
Do not place client keys in source files, logs or acceptance reports.

## Layout

The current Flutter desktop app defines the layout and interaction reference.

```text
+------------------+----------------------------+--------------------------------+------------------+
| Account          | Search / unread / +        | Avatar / name / members        | Info / search /  |
| All chats        | Avatar  Name          Time +--------------------------------+ media / members  |
| Contacts         |         Draft/preview Badge| Date / sender / message/status |                  |
| Groups / Bots    | Conversation context menu  | Unread / mention / pinned      |                  |
| Strangers        |                            +--------------------------------+                  |
| Circles          | Background audio controls  | Quote / + Emoji [Message] Send |                  |
| Collapse         |                            |                                |                  |
+------------------+----------------------------+--------------------------------+------------------+
    176 / 64 pt                300 pt                      flexible                     300 pt
```

A screen-lock request disposes account presentation and native bot windows while
account services, selected conversation and pending drafts remain session-owned.
On macOS 14 and later, bot cookies use an account-isolated persistent WebKit
profile derived from the canonical data directory and account identity. Locking
retains that profile; sign-out closes its windows and waits for data removal.
Older macOS versions use an account-session nonpersistent store. Windows WebView2
and Linux WebKitGTK use the same canonical data-directory/account isolation rule.
Signing requests require native document provenance and completed cleanup on
sign-out. Their native desktop behavior remains unverified.

## Code boundaries

- `mixin_desktop_api/src/access/`: public account handles grouped by conversation,
  message, attachment, sticker and user. The existing API entry paths reexport
  these handles; private Core services and DAOs are not exposed.
- `lib.rs`, `home.rs`: process and account composition. Feature implementation
  belongs in the corresponding module rather than accumulating in these roots.
- `navigation.rs`, `links/`: account-owned navigation, URI interpretation and
  confirmed actions. Original source conversation is preserved across navigation.
- `chat/`, `composer/`, `conversation/`, `sidebar/`, `search/`, `settings/`: feature
  presentation, bounded queries, editing buffers and error/cancellation handling.
- `security/`, `notifications/`, `playback.rs`, `transfer/`: account presentation
  coordinators over shared business handles and event streams.
- `bot/`: shared browser presentation, bridge context and account-window routing.
- `shell/`, `platform/`: process-instance handoff, native window lifecycle and
  target-specific WebKit/WebView2, notifications and tray adapters. Window bounds
  use the existing process SettingsClient, with one bounded latest-value queue.
  Bot signing and private credentials stay in Core.
- `native/webkit-extension/`: Linux's small C process-boundary adapter. It supplies
  native frame/document provenance; it contains no signing or business logic.
- `runtime.rs`: cancellable business reads, bounded subscription delivery and
  completion of user-committed writes after their awaiting view is disposed.
- `ui.rs`, `theme.json`: reusable native controls, semantic colors and layout
  tokens. Original icons, logo and chat wallpaper are retained as assets.

UI signals stay on the NSH engine thread. No NSH screen imports Core DAOs or keeps
an independent durable profile, message, conversation or settings store.

## Validation

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p mixin_desktop_ui
git diff --check
```

For explicit isolated runtime validation through the public API:

```sh
MIXIN_DESKTOP_DATA_DIR=/absolute/private/test-directory \
  cargo run -p mixin_desktop_api --example runtime_check -- --login
```

`--login` authorizes through `mixtest`'s macOS target. Omit it to restore the saved
test session. The example requires a mixtest account and does not send messages.
Live send/receive checks use a separately authorized test-only example; never use
personal accounts, commit credentials, or publish authentication URLs.

Visual behavior that has not been inspected is reported separately from source, build and runtime results.

## macOS bundle

The app uses its own native executable entry point for process ownership and
native-component registration. Build that entry point into a local bundle with:

```sh
python3 nsh/tool/package_macos.py
# Add --release for an optimized build, or --out /absolute/path/Mixin.app.
```

The packager writes the app icon, microphone/local-network purpose strings and
an alternate `mixin:` URL registration, then applies and verifies ad hoc signing.
It does not select a system-default protocol handler. Distribution signing,
notarization, updates and other platform packages need separate validation.

Windows and Linux native staging/archive tools, prerequisites and remaining runtime
checks are documented in [packaging/README.md](packaging/README.md). Their existence
does not establish native target acceptance.
