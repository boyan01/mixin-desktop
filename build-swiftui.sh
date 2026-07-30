#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
configuration="${CONFIGURATION:-Debug}"

if [[ $# -gt 0 && "$1" != -* ]]; then
  configuration="$1"
  shift
fi

build_root="$repository_root/Build"
products_dir="$build_root/Products"
intermediates_dir="$build_root/Intermediates"
derived_data_dir="$build_root/DerivedData"
source_packages_dir="$build_root/SourcePackages"
module_cache_dir="$build_root/ModuleCache.noindex"
app_path="$products_dir/$configuration/Mixin Messenger.app"

mkdir -p \
  "$products_dir" \
  "$intermediates_dir" \
  "$derived_data_dir" \
  "$source_packages_dir" \
  "$module_cache_dir"

export CLANG_MODULE_CACHE_PATH="$module_cache_dir"
export SWIFT_MODULE_CACHE_PATH="$module_cache_dir"

xcodebuild \
  -project "$repository_root/swiftui/MixinMessenger.xcodeproj" \
  -scheme MixinMessenger \
  -configuration "$configuration" \
  -destination "platform=macOS" \
  -derivedDataPath "$derived_data_dir" \
  -clonedSourcePackagesDirPath "$source_packages_dir" \
  SYMROOT="$products_dir" \
  OBJROOT="$intermediates_dir" \
  SHARED_PRECOMPS_DIR="$build_root/PrecompiledHeaders" \
  "$@" \
  build

if [[ ! -d "$app_path" ]]; then
  echo "error: build succeeded but app was not found at $app_path" >&2
  exit 1
fi

codesign --verify --deep --strict "$app_path"

if ! codesign -d --entitlements :- "$app_path" 2>/dev/null \
  | plutil -p - \
  | grep -q '"com.apple.security.app-sandbox" => true'; then
  echo "error: built app does not have App Sandbox enabled" >&2
  exit 1
fi

echo
echo "SwiftUI app built successfully:"
echo "$app_path"
echo "Code signing: valid"
echo "App Sandbox: enabled"
