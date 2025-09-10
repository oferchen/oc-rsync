#!/usr/bin/env bash
set -euo pipefail

# Ensure the repository contains no obvious placeholder tokens.
if git grep -nE 'REPLACEME|PLACEHOLDER|TBD' -- . >/dev/null; then
  echo "Placeholder token detected" >&2
  git grep -nE 'REPLACEME|PLACEHOLDER|TBD' -- .
  exit 1
fi

echo "No placeholders found"
