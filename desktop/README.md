# Swift bridge

`desktop/rust` is the UniFFI-only bridge between SwiftUI and
`mixin_desktop_api`. It must not depend on `mixin_desktop_core` directly.

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
  desktop/desktop/Generated \
  --swift-sources \
  --config desktop/uniffi-swift.toml

target/debug/uniffi-bindgen-swift \
  target/debug/libmixin_desktop.a \
  desktop/generated \
  --headers \
  --config desktop/uniffi-swift.toml

target/debug/uniffi-bindgen-swift \
  target/debug/libmixin_desktop.a \
  desktop/generated \
  --modulemap \
  --module-name MixinDesktopFFI \
  --modulemap-filename module.modulemap \
  --config desktop/uniffi-swift.toml

find desktop/desktop/Generated desktop/generated \
  -type f \
  -exec perl -pi -e 's/[ \t]+$//' {} +
find desktop/desktop/Generated desktop/generated \
  -type f \
  -exec perl -0777 -pi -e 's/\n+\z/\n/' {} +
```
