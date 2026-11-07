#!/bin/sh
set -eu
if [ "$(uname -s)" != Linux ]; then
    echo "Build this extension on Linux with WebKitGTK 6 development packages." >&2
    exit 1
fi
if [ "$#" -ne 1 ]; then
    echo "Usage: build.sh /absolute/path/to/native-executable-directory" >&2
    exit 1
fi
case "$1" in /*) ;; *) echo "Output directory must be absolute." >&2; exit 1 ;; esac
source_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir="$1/mixin-webkit-extension"
mkdir -p "$output_dir"
# shellcheck disable=SC2046
"${CC:-cc}" -std=c11 -D_POSIX_C_SOURCE=200809L -O2 -fPIC -fvisibility=hidden -Wall -Wextra -Werror \
    -shared "$source_dir/bridge.c" -o "$output_dir/libmixin-webkit-extension.so" \
    $(pkg-config --cflags --libs webkitgtk-web-process-extension-6.0)
