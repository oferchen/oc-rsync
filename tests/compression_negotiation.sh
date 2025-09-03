#!/usr/bin/env bash
set -euo pipefail

# Run compression codec negotiation tests from the compress crate
cargo test -p compress --test codecs -- negotiation_helper_picks_common_codec
cargo test -p compress --test codecs -- lzo_roundtrip

# Engine should prefer zstd when both peers support it
cargo test -p engine --test compress -- codec_selection_prefers_zstd
cargo test -p engine --test compress -- lzo_roundtrip

# Protocol handshake exchanges codec lists using Message::Codecs
cargo test -p protocol --test server -- server_negotiates_version

# Stock rsync falls back to zlib when it doesn't support codec lists
cargo test -p oc-rsync --test rsync_zlib -- rsync_client_falls_back_to_zlib
