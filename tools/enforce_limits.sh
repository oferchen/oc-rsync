#!/usr/bin/env bash
set -euo pipefail

# Enforce basic file and line length limits for tracked Rust source files.
# The script fails if a file exceeds MAX_LINES lines or contains a line
# longer than MAX_COLS characters.

MAX_LINES=2000
MAX_COLS=200
status=0

while IFS= read -r -d '' file; do
  lines=$(wc -l < "$file")
  if (( lines > MAX_LINES )); then
    echo "$file has $lines lines (limit $MAX_LINES)"
    status=1
  fi
  if grep -nE ".{$((MAX_COLS+1))}" "$file" >/dev/null; then
    echo "$file contains a line over $MAX_COLS characters"
    status=1
  fi
done < <(git ls-files -z '*.rs')

exit $status
