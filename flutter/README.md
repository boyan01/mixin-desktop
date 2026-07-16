# Mixin Desktop Flutter UI

Flutter desktop UI backed by the shared Rust core through
`flutter_rust_bridge`.

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
