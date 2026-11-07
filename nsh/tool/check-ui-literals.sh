#!/usr/bin/env bash
# Reports raw geometry and color literals in screen modules, per file. Screens use
# `ui::` components and tokens (see nsh/DESIGN.md). Report only: it always exits 0.
set -euo pipefail
cd "$(dirname "$0")/.."

find src -name '*.rs' \
  -not -path 'src/ui/*' \
  -not -path 'src/platform/*' \
  -not -path '*/tests/*' \
  -not -name 'tests.rs' \
  -not -name '*_tests.rs' |
  while read -r file; do
    awk -v file="$file" '
      /#\[cfg\(test\)\]/ { exit }
      /^[[:space:]]*\/\// { next }
      {
        line = $0; px += gsub(/(^|[^a-z_.])px\(/, "&", line)
        line = $0; rgb += gsub(/(^|[^a-z_])(rgba?|hsla?)\(/, "&", line)
        line = $0; hex += gsub(/#[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]/, "&", line)
        line = $0; rounded += gsub(/\.rounded_[a-z0-9]+\(/, "&", line)
      }
      END {
        total = px + rgb + hex + rounded
        if (total > 0) printf "%d\t%d\t%d\t%d\t%d\t%s\n", total, px, rgb, hex, rounded, file
      }
    ' "$file"
  done |
  sort -k1,1nr -k6,6 |
  awk -F '\t' '
    BEGIN { printf "%6s %5s %4s %4s %8s  %s\n", "total", "px", "rgb", "hex", "rounded", "file" }
    {
      printf "%6d %5d %4d %4d %8d  %s\n", $1, $2, $3, $4, $5, $6
      total += $1; px += $2; rgb += $3; hex += $4; rounded += $5; files += 1
    }
    END { printf "%6d %5d %4d %4d %8d  %d files\n", total, px, rgb, hex, rounded, files }
  '
