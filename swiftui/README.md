# Mixin Messenger SwiftUI

`swiftui/rust` is the UniFFI-only bridge between SwiftUI and
`mixin_desktop_api`. It must not depend on `mixin_desktop_core` directly.

## Run the SwiftUI app

The SwiftUI app and the Flutter app use the same bundle identifier and macOS
App Sandbox container. Build the runnable Debug app with code signing enabled
so `NSDocumentDirectory` resolves to the existing container:

```sh
./build-swiftui.sh

'./Build/Products/Debug/Mixin Messenger.app/Contents/MacOS/Mixin Messenger'
```

Pass `Release` as the first argument to create a Release build. Xcode and
SwiftPM build products and intermediates are kept under the repository-level
`Build/` directory. The Rust bridge keeps using the same Cargo `target/`
directory as Xcode.

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

`bridgegen/exports.txt` records only the existing Swift export surface and the
receiver to call. Method signatures, DTO fields, and unit enum variants are read
from the public Rust API and its current re-exported type definitions. For example:

```text
account SwiftAccountHandle ConversationAccess self.client.conversation() conversations
model record AccountProfile
```

A signature or field change needs no matching declaration edit in the bridge.
To expose a new method, add its name to the allowlist; use `swift_name=rust_name`
only when preserving a different Swift-facing name. This explicit selection keeps
new public Rust operations from silently becoming UI exports.

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
hand-written bridge. Flutter/FRB and Tauri retain their existing integration.
