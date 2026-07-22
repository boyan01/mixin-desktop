# Historical parity lessons

These are search anchors from earlier successful ports, not permanent specifications. Verify each against the current source before implementation.

## Shell and navigation

- Previous source anchors included `kResponsiveNavigationMinWidth = 320`, a 300-pixel conversation list, sidebar widths from 64 to 176, and route switching near 620 pixels.
- Previous desktop startup used a 384 by 480 minimum window, a 1280 by 750 Windows default, and a hidden macOS title bar.
- Lifecycle-aware ticker, focus, global shortcuts, title-bar behavior, and platform-specific branches are part of parity, not cosmetic extras.

## Login and state ownership

- The prior login card was 520 by 418, but verify current layout values.
- QR authorization waiting belongs in Rust through `LoginHandle.wait()` with synchronous cancellation. Flutter retains disposal and supersession guards.
- Verify that scan, provisioning, success, timeout, retry, and transport-error states all have reachable transitions. Do not collapse a request failure into an empty "not scanned yet" result.
- `AccountRuntime` owns the profile snapshot, persistence, and change stream. Do not restore a Flutter-side profile cache or one-second polling.
- Prefer a tree-scoped session owner over an app-global nullable account object.

## Lists, pages, and refresh behavior

- The conversation list previously used 15-row bounded windows, prefetch near visible indices, and eviction of far-away rows. Do not replace it with load-all behavior.
- Conversation changes should wake Flutter through streams rather than timers.
- ChatInfo and shared-app flows should render local conversation/cache data first and refresh remote details in the background. A remote refresh failure must not hide usable cached content.
- Preserve the source widget vocabulary. Earlier drift came from substituting generic `AppBar` and `ListTile` for source-specific app bars and grouped cells.

## Messages and compatibility

- Compare each message category and nested state propagation. Previous bugs involved image captions, video clipping, audio/sticker status placement, file/post previews, pinned spacing, APP_CARD, APP_BUTTON_GROUP, location arrows, and disappearing-message state.
- Determine caption visibility using the same trimmed-empty rule while preserving the raw displayed value when that is what the source does.
- Device-transferred records may contain known nullable or empty media status. Map only documented compatibility values; reject arbitrary invalid strings.
- Conversation open/load/mutation failures must log operation context, exception, and stack trace so blank UI states remain diagnosable.

## Core versus platform shell

- Rust core may filter and emit reusable `NotificationEvent` values, including mute, mention, quote, and recall rules.
- Flutter/native shell owns app lifecycle suppression, current-conversation suppression, privacy preview text, permissions, OS delivery, click navigation, and dismissal.
- Use the same split for other features: reusable business decision in core; platform identity and interaction in the shell.

## FRB and validation failure patterns

- Stabilize Rust APIs before code generation. Stale generated bindings previously left removed methods in Dart and caused misleading compile failures.
- Return bridge-safe DTOs and primitives; exposing `PathBuf` previously generated an unwanted opaque type.
- Automated tests do not prove runtime parity. Startup acknowledgements, SQL parameter behavior, Signal refresh, native plugins, and visual flow have previously required live runs.
- A macOS `open returned 1` warning is not automatically an app failure if the process, VM Service, DevFS, window, and runtime logs are healthy.
- If native builds fail, verify the selected CocoaPods executable before changing repository code; a stale `/usr/local/bin/pod` shim has previously masked a working Homebrew installation.
