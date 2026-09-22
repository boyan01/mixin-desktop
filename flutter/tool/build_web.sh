#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
wasm_host=$(rustc -vV | sed -n 's/^host: //p')
export AR_wasm32_unknown_unknown="${AR_wasm32_unknown_unknown:-$(rustc --print sysroot)/lib/rustlib/$wasm_host/bin/llvm-ar}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target/flutter-web}"
command -v wasm-bindgen >/dev/null || { echo 'Install wasm-bindgen-cli matching Cargo.lock.' >&2; exit 1; }
export RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--import-memory -C link-arg=--max-memory=4294967296 -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base'
cargo +nightly build --locked -p mixin_desktop_flutter --target wasm32-unknown-unknown -Z build-std=std,panic_abort
mkdir -p flutter/build/rust-web flutter/web/pkg
wasm-bindgen "$CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/mixin_desktop_flutter.wasm" \
  --target web --out-dir flutter/build/rust-web
npm ci --prefix flutter/tool/web
node flutter/tool/web/bundle.mjs
cd flutter
flutter build web --no-wasm-dry-run "$@"
