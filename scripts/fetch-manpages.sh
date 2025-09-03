#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/.."
cd "$ROOT_DIR"

scripts/fetch-rsync.sh
cp rsync-3.4.1/rsync.1 docs/spec/rsync.1
cp rsync-3.4.1/rsyncd.conf.5 docs/spec/rsyncd.conf.5
