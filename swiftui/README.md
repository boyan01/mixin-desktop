# Mixin Messenger SwiftUI

`swiftui/rust` is the UniFFI-only bridge between SwiftUI and
`mixin_desktop_api`. It must not depend on `mixin_desktop_core` directly.

## Run the SwiftUI app

SwiftUI, Flutter, and Tauri share the same bundle identifier per build mode:
Debug uses `one.mixin.messenger.desktop.debug`; Release retains
`one.mixin.messenger.desktop`. Flutter Profile also uses the Debug identifier.
Signed sandboxed builds share a container within each mode, while Debug and
Release keep separate accounts and data. The first Debug launch uses a new
container; existing Release data is not migrated or copied.

Build the runnable Debug app with code signing enabled so
`NSDocumentDirectory` resolves inside its app container:

```sh
./build-swiftui.sh

'./Build/Products/Debug/Mixin Messenger.app/Contents/MacOS/Mixin Messenger'
```

Pass `Release` as the first argument to create a Release build. Xcode and
SwiftPM build products and intermediates are kept under the repository-level
`Build/` directory. The Rust bridge keeps using the same Cargo `target/`
directory as Xcode.

For Tauri, use `npm run tauri -- dev` or `npm run tauri -- build --debug` from
`typescript/` for the Debug identifier. `npm run tauri -- build` and
`npm run tauri -- dev --release` retain the Release identifier. The npm entry
point selects the macOS identifier automatically; invoking the Tauri CLI directly
bypasses this selection. To test the selection, run
`node --test scripts/tauri.test.mjs` from `typescript/`.

`CODE_SIGNING_ALLOWED=NO` is suitable for compile-only validation. Do not run
that product: an unsigned process resolves the data directory outside the app
container and starts without the saved account.

Quit the Flutter app before starting the SwiftUI app. Both targets own the same
account runtime and database files, so they must not run concurrently.

The generated Swift source, C header, and module map are committed so the Xcode
project remains inspectable from a clean checkout. Xcode regenerates them before
compiling the Swift sources.

Generate all adapters and bindings from the repository root:

```sh
./swiftui/generate-bindings.sh
```

Check that committed Rust adapters, Swift sources, C headers, and the module map
match their inputs without rewriting them:

```sh
./swiftui/generate-bindings.sh Debug --check
```

## Rust adapter generation

`bridgegen` is a standalone development tool. It parses Rust with `syn` and does
not link the application, so it can regenerate stale adapters before the bridge
compiles. Xcode runs it before building Rust and generating Swift bindings.
The UniFFI CLI binary also belongs to this tool package. Building it with
`--features swift-bindgen` does not rebuild the application bridge with CLI
dependencies enabled.

`bridgegen/exports.txt` records only the existing Swift export surface and the
receiver to call. Method signatures, DTO fields, and unit enum variants are read
from the public Rust API and its current re-exported type definitions. For example:

```text
account SwiftAccountHandle ConversationAccess self.client.conversation() conversations
model record AccountProfile
model record MessageItem=MessageListView
```

A signature or field change needs no matching declaration edit in the bridge.
To expose a new method, add its name to the allowlist; use `swift_name=rust_name`
only when preserving a different Swift-facing name. This explicit selection keeps
new public Rust operations from silently becoming UI exports.
Records and enums support the same `swift_name=rust_name` spelling. It changes
the generated Swift name while still using the original Rust type and all its
fields. Documentation comments on types, fields, and variants are preserved;
conditional fields and variants still require explicit adapter support.

Regenerate or check just the Rust adapters with:

```sh
cargo run -p mixin_swift_bridgegen
cargo run -p mixin_swift_bridgegen -- --check
cargo test -p mixin_swift_bridgegen
```

Generated adapters live in `rust/src/generated/` and are committed. Do not edit
them. The generator supports shared receivers, owned bridge-safe values,
`ClientResult<T>` / `Result<T, ClientError>`, same-shape records, and unit enums.
Unsupported signatures fail generation and require an explicit adapter decision.

Keep subscription lifecycle, cancellation, object construction, typed error
lowering, validation, batching, and presentation-specific conversions in the
hand-written bridge. `rust/src/subscription.rs` owns stream cancellation and a
small macro for ordinary typed subscriptions. Notification error conversion stays
explicit. Cancellation releases both an active `next()` and reads queued behind it.

All three UI bridges now use the full public `MessageListView` read model. Swift
keeps the name `MessageItem`; TypeScript keeps `Message`. Flutter already exposes
the complete type. Do not add another field-by-field message copy in a UI bridge.
