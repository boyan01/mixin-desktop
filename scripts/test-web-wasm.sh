#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# LLVM's archiver handles ring's WASM objects; Apple's ar produces an empty archive.
wasm_host=$(rustc -vV | sed -n 's/^host: //p')
wasm_llvm_ar="$(rustc --print sysroot)/lib/rustlib/$wasm_host/bin/llvm-ar"
export AR_wasm32_unknown_unknown="${AR_wasm32_unknown_unknown:-$wasm_llvm_ar}"
if [[ ! -x "$AR_wasm32_unknown_unknown" ]]; then
  echo 'Install rustup component llvm-tools, or set AR_wasm32_unknown_unknown to llvm-ar.' >&2
  exit 1
fi
command -v wasm-bindgen >/dev/null || {
  echo 'Install wasm-bindgen-cli matching the wasm-bindgen version in Cargo.lock.' >&2
  exit 1
}
command -v protoc >/dev/null || [[ -n "${PROTOC:-}" ]] || {
  echo 'Install protoc, or set PROTOC to its executable path.' >&2
  exit 1
}

cargo check --locked -p mixin_desktop_api --target wasm32-unknown-unknown --lib
cargo build --locked -p mixin_desktop_core --example browser_contract \
  --features browser-tests --target wasm32-unknown-unknown
wasm-bindgen "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/debug/examples/browser_contract.wasm" \
  --target web --out-dir mixin_desktop_core/web/pkg
npm ci --prefix mixin_desktop_core/web
npm test --prefix mixin_desktop_core/web
