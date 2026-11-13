# Mixin Messenger · Native Shell

The native Rust UI depends on `mixin_desktop_api`. Protocol, credentials, SQLite,
Signal, Blaze, sync, jobs, secure screen-lock credentials, media preparation and
durable settings remain in the shared Rust business layer. NSH owns presentation,
navigation, disposable editing state and native platform adapters.

The migration from the Flutter desktop app (`/Users/yangbin/workspace/mixin/flutter-app`)
is in progress. See [docs/parity.md](../docs/parity.md) for per-feature status and
[ROADMAP.md](ROADMAP.md) for remaining work.

## Run

NSH comes from the `main` branch of `longbridge/nsh`; `Cargo.lock` pins the commit
and `cargo update -p nsh-core` moves it to the latest. Use Rust and the platform's
native SDK. From the repository root:

```sh
cargo run -p mixin_desktop_ui
```

The native executable opens the Flutter desktop app's data directory in place and
reads and writes it in the Flutter formats, so either app can open the same data:

- macOS: `~/Library/Containers/one.mixin.messenger.desktop/Data/Documents`
- Windows: `Documents\Mixin`
- Linux: `~/.mixin`

`mixin.db`, `fts.db`, and `signal.db` stay at the Flutter drift schema versions.
Credentials, Signal counters, the screen passcode, and shared preferences live in
the Flutter Hive boxes; `app.db` holds NSH-only preferences. While open, NSH holds
the `hydrated_box` Hive lock, so it refuses to start while the Flutter app runs.
Use `MIXIN_DESKTOP_DATA_DIR` to select an isolated test directory.

On macOS, `cargo run` starts an unsandboxed process, which macOS asks to approve
before it reads another app's container. The sandboxed bundle below opens the
container directly, like a local Flutter build.

The NSH CLI bypasses the native executable entry point. Prefer `cargo run` when
validating startup, process ownership, native component registration and shutdown.
macOS is the first validation target. Windows and Linux are unverified; see the roadmap.

## Shared development builds

The repository's `.cargo/config.toml` owns the native development profile for
ordinary `cargo test` and the preview helper's generated NSH workspaces. Both use
the root `target/` by default. Project crates retain line-table debugging and
incremental compilation, even when NSH treats them as dependencies. Third-party
crates omit debugging information and incremental caches. The four optimized
dependencies match NSH's generated native profile. The test profile inherits
these settings; release profiles are unchanged.
`quickjs-jit-macro` keeps its symbols because stripping its embedded native code
can produce a Mach-O library that the macOS loader rejects. Its Rust debugging
information remains disabled.

The helper also seeds each generated workspace's lockfile from the repository's
`Cargo.lock`. Cargo retains compatible pinned versions and adds the host-only
dependencies. It reseeds only when the source lockfile changes or the generated
lockfile is missing, so repeated previews retain their host dependency versions.
This avoids independently upgrading the preview's shared dependencies.

The capture helper passes the shared target directory because this NSH CLI
otherwise selects a target directory below the preview entry. A caller's
`CARGO_TARGET_DIR` override is respected and relative overrides are resolved from
the caller's directory. Use `python3 nsh/tool/preview.py run` from the repository
root for an interactive preview with this same cache. Calling `nsh run` directly
still uses the CLI's entry-local target directory unless `CARGO_TARGET_DIR` is set.

Sharing a directory and profile avoids duplicate artifacts when the resolved
crate, version and features agree. NSH currently turns its pinned Git checkout
into path dependencies for generated hosts; ordinary tests still use the Git
source. Those NSH artifacts, test executables and different feature combinations
can remain separate. Do not edit generated `.nsh/rust/Cargo.toml` files or change
the pinned NSH source just to remove those differences.

## Button design previews

The native preview browser includes a workflow selector, previous/next controls,
and offline page/dialog scenes for framed buttons. See
[the surface inventory and capture commands](preview/README.md).
`create-group` shows the contact list, then exposes the selected-avatar strip and
header Next action only after a choice. `message-forward` uses the same visual
parts with immediate single-target sending. Both lists scroll without page controls.
The `Select`, `Next` and `Error` preview actions expose the group states.

## Markdown previews

The debug-only preview browser renders production components over offline fixtures:

```sh
python3 nsh/tool/preview.py run
```

Select `post-reader` for headings, mixed Chinese/English prose, lists, task lists
and quotes, or `post-blocks` for highlighted code and tables. `Larger text`
changes the real message font setting in memory; no account data is read or saved.
The existing `messages-cards` scene includes the bounded Post bubble.

From the repository root, capture the native macOS reader at a specific appearance
and width with:

```sh
python3 nsh/tool/preview.py capture post-reader --label light --locale zh-CN
python3 nsh/tool/preview.py capture post-blocks --label dark-narrow --scheme dark --width 480
python3 nsh/tool/preview.py capture post-reader --action "Larger text" --label large
```

Posts and downloaded Markdown attachments share the reader in `src/ui/markdown.rs`.
The reading column is capped at `post-reader-width`; its text scales with the chat
font preference. Code and tables retain their native horizontal scroll views.

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
- `shell/`: process-instance handoff and native window lifecycle. Window bounds
  use the existing process SettingsClient, with one bounded latest-value queue.
- Target-specific adapters sit beside the feature they serve, as `macos`,
  `windows` and `linux` child modules: `shell/` (URL capture, tray), `bot/`
  (WebKit/WebView2 hosts) and `notifications/native/`. Bot signing and private
  credentials stay in Core.
- `native/webkit-extension/`: Linux's small C process-boundary adapter. It supplies
  native frame/document provenance; it contains no signing or business logic.
- `runtime.rs`: cancellable business reads, bounded subscription delivery and
  completion of user-committed writes after their awaiting view is disposed.
- `ui/`, `theme.json`, `DESIGN.md`: reusable native controls, semantic colors and layout
  tokens; `ui::parts` holds the page and dialog parts shared across features.
  Original icons, logo and chat wallpaper are retained as assets.

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

Build with the official NSH CLI from the repository root:

```sh
nsh package nsh --target macos --id one.mixin.messenger.desktop -o nsh/.nsh/out/macos
```

The CLI builds an optimized app and applies ad hoc signing. `nsh.json` declares
its icon, sandbox entitlements, usage descriptions and `mixin:` URL scheme.
The bundle id `one.mixin.messenger.desktop` preserves the Flutter container;
a sandboxed `MIXIN_DESKTOP_DATA_DIR` must point inside that container.
For isolated validation, use a separate bundle id and output directory.

The application owns initialization, single-instance forwarding and shutdown
through the official runner's lifecycle APIs. Notification audio is installed in
the app's container `Library/Sounds`.

Isolated startup, logging, second-instance forwarding and signal-driven shutdown
have been verified. Close/Quit menu interactions and audible notification delivery
remain unverified. Distribution signing, notarization and updates need separate
validation.

Windows and Linux native staging/archive tools, prerequisites and remaining runtime
checks are documented in [packaging/README.md](packaging/README.md). Their existence
does not establish native target acceptance.
