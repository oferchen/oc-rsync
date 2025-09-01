#!/usr/bin/env bash
set -euo pipefail

# Execute daemon authentication tests individually for clarity
cargo test --test daemon -- --exact daemon_rejects_invalid_token
cargo test --test daemon -- --exact daemon_rejects_unauthorized_module
cargo test --test daemon -- --exact daemon_accepts_authorized_client
cargo test --test daemon -- --exact client_authenticates_with_password_file
cargo test --test daemon -- --exact client_respects_no_motd
cargo test --test daemon -- --exact daemon_logs_connections
cargo test --test daemon -- --exact daemon_honors_bwlimit
