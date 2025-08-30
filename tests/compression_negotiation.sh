#!/usr/bin/env bash
set -euo pipefail

# Run compression codec negotiation tests from the compress crate
cargo test -p compress --test codecs --features blake3 -- negotiation_helper_picks_common_codec

# Engine should prefer zstd when both peers support it
cargo test -p engine --test compress --features blake3 -- codec_selection_prefers_zstd

# Protocol handshake exchanges codec lists using Message::Codecs
cargo test -p protocol --test server --features blake3 -- server_negotiates_version

# Stock rsync falls back to zlib when it doesn't support codec lists
cargo test -p oc-rsync --test rsync_zlib --features blake3 -- rsync_client_falls_back_to_zlib
