#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

configuration="${1:-Debug}"
check="${2:-}"
if [[ "$configuration" != Debug && "$configuration" != Release ]] ||
   [[ -n "$check" && "$check" != --check ]] || [[ $# -gt 2 ]]; then
  echo "usage: $0 [Debug|Release] [--check]" >&2
  exit 1
fi

if [[ "$check" == --check ]]; then
  cargo run -p mixin_swift_bridgegen -- --check
else
  cargo run -p mixin_swift_bridgegen
fi

profile=debug
if [[ "$configuration" == Release ]]; then
  profile=release
  cargo build -p mixin_desktop_swift --release
else
  cargo build -p mixin_desktop_swift
fi
cargo build -p mixin_swift_bridgegen --features swift-bindgen --bin uniffi-bindgen-swift

target_dir="${CARGO_TARGET_DIR:-$repository_root/target}"
bindgen="$target_dir/debug/uniffi-bindgen-swift"
library="$target_dir/$profile/libmixin_desktop.a"
config="$repository_root/swiftui/uniffi-swift.toml"
swift_output="$repository_root/swiftui/MixinMessenger/Generated"
ffi_output="$repository_root/swiftui/generated"

if [[ "$check" == --check ]]; then
  temporary_output="$(mktemp -d)"
  trap 'rm -rf "$temporary_output"' EXIT
  swift_output="$temporary_output/swift"
  ffi_output="$temporary_output/ffi"
fi
mkdir -p "$swift_output" "$ffi_output"
"$bindgen" "$library" "$swift_output" --swift-sources --config "$config"
"$bindgen" "$library" "$ffi_output" --headers --config "$config"
"$bindgen" "$library" "$ffi_output" --modulemap --module-name MixinDesktopFFI \
  --modulemap-filename module.modulemap --config "$config"
find "$swift_output" "$ffi_output" -type f -exec perl -pi -e 's/[ \t]+$//' {} +
find "$swift_output" "$ffi_output" -type f -exec perl -0777 -pi -e 's/\n+\z/\n/' {} +

if [[ "$check" == --check ]]; then
  diff -ru "$repository_root/swiftui/MixinMessenger/Generated" "$swift_output"
  diff -ru "$repository_root/swiftui/generated" "$ffi_output"
fi
