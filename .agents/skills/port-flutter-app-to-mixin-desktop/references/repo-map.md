# Repository map

## Resolve checkouts

Resolve the destination from the active repository and the source from per-clone Git configuration:

```sh
DEST_ROOT="$(git rev-parse --show-toplevel)"
SOURCE_ROOT="$(git config --local --get mixinDesktop.flutterAppRoot)"
```

If `SOURCE_ROOT` is missing or invalid, ask the user for the source checkout and configure it once:

```sh
git config --local mixinDesktop.flutterAppRoot /path/to/flutter-app
```

Do not commit a machine-specific path, scan broad filesystem roots, or silently fall back to a guessed location. Validate both roots before use:

```sh
test -f "$DEST_ROOT/Cargo.toml"
test -f "$DEST_ROOT/flutter/pubspec.yaml"
test -f "$SOURCE_ROOT/pubspec.yaml"
test -f "$SOURCE_ROOT/lib/main.dart"
```

Destination paths below are relative to `$DEST_ROOT`:

- Flutter app: `flutter/`
- Rust core: `mixin_desktop_core/`
- Mixin SDK/runtime support: `mixin-bot-sdk/`
- FRB adapter crate: `flutter/rust/`
- FRB configuration: `flutter/flutter_rust_bridge.yaml`
- Generated Dart bindings: `flutter/lib/src/rust/`

## Destination layers

- `flutter/lib/main.dart` and `flutter/lib/app.dart`: startup, global scopes, platform lifecycle, and session mounting.
- `flutter/lib/pages/`: page-level composition and routing.
- `flutter/lib/widgets/`: source-parity presentation widgets.
- `flutter/lib/controllers/`: view/session orchestration, bounded windows, interaction state, and DTO mapping.
- `flutter/lib/src/rust/desktop_api.dart`: convenient imports/types around generated bindings.
- `flutter/rust/src/api/`: thin FRB-visible account, login, desktop, and logging adapters.
- `mixin_desktop_core/src/runtime/`: account-scoped behavior and coarse access objects.
- `mixin_desktop_core/src/db/`: SQLite persistence and queries.

## Source discovery

Start with the corresponding source path and follow imports rather than assuming one-to-one filenames:

- `lib/main.dart`, `lib/app.dart`
- `lib/ui/landing/`
- `lib/ui/home/`
- `lib/ui/setting/`
- `lib/widgets/`
- provider/notifier files reached by those widgets
- DAO/API/service code reached by those providers
- source tests for edge semantics

Useful discovery commands:

```sh
rg -n "WidgetName|methodName|message category" "$SOURCE_ROOT/lib"
rg -n "WidgetName|methodName|message category" "$DEST_ROOT/flutter/lib" "$DEST_ROOT/mixin_desktop_core/src" "$DEST_ROOT/flutter/rust/src"
rg --files "$SOURCE_ROOT/test" "$DEST_ROOT/flutter/test"
```

## Bridge workflow

From `$DEST_ROOT/flutter`:

```sh
flutter_rust_bridge_codegen generate
flutter analyze
flutter test
flutter build macos --debug
```

From `$DEST_ROOT`:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets
git diff --check
```

Run `cargo clippy` for the touched crate or workspace when practical. Report pre-existing unrelated lints instead of broadening a parity patch.
