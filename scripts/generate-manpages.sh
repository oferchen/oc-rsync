#!/usr/bin/env bash
set -euo pipefail

pandoc -s -t man docs/cli.md -o man/rsync-rs.1
pandoc -s -t man docs/daemon.md -o man/rsync-rs.conf.5

# Generate shell completions
cargo run --quiet --bin gen-completions -- man
mv man/_rsync-rs man/rsync-rs.zsh
