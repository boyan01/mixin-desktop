# Mixin Desktop Flutter UI

Flutter desktop UI backed by the shared Rust core through
`flutter_rust_bridge`.

## Architecture

- Copy presentation widgets, theme, assets, and localization from
  `mixin/flutter-app` as faithfully as possible.
- Keep Flutter controllers limited to view state, paging windows, and mapping
  coarse Rust DTOs into presentation models.
- Keep API, Blaze, Signal, sync jobs, expiration, and SQLite ownership in
  `mixin_desktop_core`.
- Do not copy Drift DAOs, Riverpod data providers, SDK clients, or background
  workers into this package. Add a Rust bridge operation instead.
- Unsupported actions must be visibly disabled until their Rust operation is
  available; do not add local mock behavior or hidden fallbacks.

```text
Flutter widgets -> controllers/view models -> flutter_rust_bridge -> Rust core
```

## Run

Install Flutter, Rust, CocoaPods, and `flutter_rust_bridge_codegen`, then run:

```sh
flutter pub get
flutter_rust_bridge_codegen generate
flutter run -d macos
```

## Web (experimental)

Flutter Web and the Rust WASM bridge build successfully. The browser runtime
runs in one dedicated Worker; SQLite and files use OPFS on the current origin.
The JavaScript build uses checked conversions at FRB's `BigInt`/`int` boundary.
Values outside JavaScript's exact integer range fail instead of rounding.

Prerequisites: Flutter, Node.js, Rust nightly with `rust-src`, stable Rust's
`llvm-tools` component, `protoc`, and `wasm-bindgen-cli` matching Cargo.lock.
Set `PROTOC` if the bundled protobuf compiler does not work on the host.
The pinned [Signal fork](https://github.com/boyan01/libsignal/tree/ec54aa3a01159edc4fa9ccb80748b450d51788ab/rust/protocol/compatibility)
keeps the original Signal protocol while updating its cryptographic, serialization
and RNG dependencies. Cross-version checks cover keys, signatures, prekey
handshakes, sessions and group messages. No local `getrandom` patch is needed.

From the repository root:

```sh
cd flutter
flutter pub get --enforce-lockfile
cd ..
./flutter/tool/build_web.sh
node flutter/tool/web/serve.mjs
```

Open `http://127.0.0.1:7357`. The build script emits the Rust bundle under
`flutter/web/pkg/` before building Flutter, and the local server supplies the
cross-origin isolation headers needed by shared WASM memory. Build artifacts and
npm dependencies are ignored. The Rust artifact currently uses the debug profile.

For Flutter development, after building the Rust bundle:

```sh
cd flutter
flutter run -d chrome \
  --web-header=Cross-Origin-Opener-Policy=same-origin \
  --web-header=Cross-Origin-Embedder-Policy=require-corp
```

Production hosting requires HTTPS and the same headers. Browser storage is
origin-specific; changing the port creates a different local data store. Only one
active core Worker may own that store at a time.

### Verified runtime contract

With Google Chrome installed, run the non-UI test in a temporary browser profile:

```sh
cd flutter
dart compile js tool/web_contract.dart -o build/web/web_contract.js
node tool/web/test.mjs
flutter test --platform chrome test/rust_int_test.dart
```

This exercises generated Dart bindings, concurrent runtime startup, SQLite
settings, an event stream, explicit native-capability errors, and persistence
across page reloads. It does not sign in or mutate a remote account.

### Remaining browser limitations

Native audio/recording, LAN device transfer, MCP, desktop notifications, tray and
OS protocol registration are unavailable. Relevant native actions are disabled;
bridge calls reject unsupported operations explicitly. Other signed-in flows,
including attachment picking/display/export, still need browser validation.

Real login and messaging are **not verified**. HTTP requests are subject to CORS,
and Blaze needs a deployment-supported authentication connector in the core
Worker: browser WebSockets cannot set the native Authorization header. The current
bundle deliberately does not guess a token URL or authentication subprotocol.
See the shared core's [browser runtime boundaries](../mixin_desktop_core/web/README.md).
The non-UI checks above do not establish production login or messaging support.

## Verify

```sh
flutter analyze
flutter test
flutter widget-preview start
```
