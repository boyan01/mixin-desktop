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

## Verify

```sh
flutter analyze
flutter test
flutter widget-preview start
```
