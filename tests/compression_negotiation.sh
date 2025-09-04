#!/usr/bin/env bash
set -euo pipefail

# Run compression codec negotiation tests from the compress crate
cargo nextest run --no-fail-fast -p compress --test codecs -- --exact negotiation_helper_picks_common_codec

# Engine should prefer zstd when both peers support it
cargo nextest run --no-fail-fast -p engine --test compress -- --exact codec_selection_prefers_zstd

# Protocol handshake exchanges codec lists using Message::Codecs
cargo nextest run --no-fail-fast -p protocol --test server -- --exact server_negotiates_version

# Stock rsync falls back to zlib when it doesn't support codec lists
cargo nextest run --no-fail-fast -p oc-rsync --test rsync_zlib -- --exact rsync_client_falls_back_to_zlib
