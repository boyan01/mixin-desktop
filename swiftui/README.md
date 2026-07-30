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

Generate the bindings manually from the repository root:

```sh
cargo build -p mixin_desktop_swift
cargo build -p mixin_desktop_swift \
  --features swift-bindgen \
  --bin uniffi-bindgen-swift

target/debug/uniffi-bindgen-swift \
  target/debug/libmixin_desktop.a \
  swiftui/MixinMessenger/Generated \
  --swift-sources \
  --config swiftui/uniffi-swift.toml

target/debug/uniffi-bindgen-swift \
  target/debug/libmixin_desktop.a \
  swiftui/generated \
  --headers \
  --config swiftui/uniffi-swift.toml

target/debug/uniffi-bindgen-swift \
  target/debug/libmixin_desktop.a \
  swiftui/generated \
  --modulemap \
  --module-name MixinDesktopFFI \
  --modulemap-filename module.modulemap \
  --config swiftui/uniffi-swift.toml

find swiftui/MixinMessenger/Generated swiftui/generated \
  -type f \
  -exec perl -pi -e 's/[ \t]+$//' {} +
find swiftui/MixinMessenger/Generated swiftui/generated \
  -type f \
  -exec perl -0777 -pi -e 's/\n+\z/\n/' {} +
```
