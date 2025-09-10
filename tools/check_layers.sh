#!/usr/bin/env bash
set -euo pipefail

# Detect overly deep crate paths that might indicate layering issues.
# We flag any `use crate::` path with five or more segments.

if git grep -n 'crate::[^;]*::[^;]*::[^;]*::[^;]*::' -- '*.rs' >/dev/null; then
  echo "Layer violation detected: overly deep crate path" >&2
  git grep -n 'crate::[^;]*::[^;]*::[^;]*::[^;]*::' -- '*.rs'
  exit 1
fi

echo "Layer check passed"
