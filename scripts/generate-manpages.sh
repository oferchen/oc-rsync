#!/usr/bin/env bash
set -euo pipefail

pandoc -s -t man docs/cli.md -o man/oc-rsync.1
pandoc -s -t man docs/daemon.md -o man/oc-rsync.conf.5

# Generate shell completions
cargo run --quiet --bin gen-completions -- man
mv man/_oc-rsync man/oc-rsync.zsh
