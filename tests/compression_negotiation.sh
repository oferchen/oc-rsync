#!/usr/bin/env bash
set -euo pipefail

# Run compression codec negotiation tests from the compress crate
cargo test -p compress --test codecs -- negotiation_helper_picks_common_codec
