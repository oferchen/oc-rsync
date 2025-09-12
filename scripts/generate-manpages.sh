#!/usr/bin/env bash
set -euo pipefail

pandoc -s -t man docs/cli.md -o man/oc-rsync.1
pandoc -s -t man docs/daemon.md -o man/oc-rsyncd.conf.5
pandoc -s -t man docs/oc-rsyncd.md -o man/oc-rsyncd.8
