#!/usr/bin/env bash
set -euo pipefail

# Run compression codec negotiation tests from the compress crate
cargo test -p compress --test codecs -- negotiation_helper_picks_common_codec

# Engine should prefer zstd when both peers support it
cargo test -p engine --test compress -- codec_selection_prefers_zstd

# Protocol handshake exchanges codec lists using Message::Codecs
cargo test -p protocol --test server -- server_negotiates_version
