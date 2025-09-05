#!/usr/bin/env bash
# Remove project banner from oc-rsync help output to make results deterministic.
# Reads from stdin and writes sanitized text to stdout.
set -euo pipefail
sed '1,3d'
