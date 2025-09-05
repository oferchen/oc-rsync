# FLAKES

This file documents tests marked `#[ignore]` and why they remain excluded from automated runs.

| Test | Reason |
| ---- | ------ |
| `tests/packaging.rs::service_unit_matches_spec` | Service unit packaging checks require systemd packaging context and are not essential for core sync behavior. |
| `tests/log_file.rs::log_file_writes_messages` | Logging to files under development; output not yet deterministic. |
| `tests/log_file.rs::log_file_format_json_writes_json` | JSON log file format still evolving. |
| `tests/log_file.rs::out_format_writes_custom_message` | Custom `--out-format` handling not finalized. |
| `tests/log_file.rs::out_format_supports_all_escapes` | Escape sequence processing incomplete. |
| `tests/log_file.rs::out_format_escapes_match_rsync` | Requires parity with upstream `rsync` binary. |
| `tests/fuzzy.rs::fuzzy_transfers_file` | Fuzzy matching feature pending implementation. |
| `tests/daemon_config.rs::daemon_config_rsync_client` | Daemon configuration interop not complete. |
| `tests/daemon_config.rs::daemon_config_authentication` | Authentication against daemon not implemented. |
| `tests/daemon_config.rs::daemon_config_motd_suppression` | MOTD suppression support incomplete. |
| `tests/daemon_config.rs::daemon_config_module_secrets_file` | Secrets file handling pending. |
| `tests/daemon_sync_attrs.rs::daemon_preserves_xattrs` | Requires extended attribute support and `libacl` which is unavailable in this environment. |
| `tests/daemon_sync_attrs.rs::daemon_preserves_xattrs_rr_daemon` | Same as above. |
| `tests/daemon_sync_attrs.rs::daemon_excludes_filtered_xattrs` | Same as above. |
| `tests/daemon_sync_attrs.rs::daemon_excludes_filtered_xattrs_rr_client` | Same as above. |
| `tests/daemon.rs::daemon_allows_path_traversal_without_chroot` | Daemon chroot/security features unfinished. |
| `tests/daemon.rs::daemon_allows_module_access` | Daemon module access tests require full daemon implementation. |
| `tests/daemon.rs::daemon_parses_secrets_file_with_comments` | Secrets file parsing not yet stable. |
| `tests/daemon.rs::client_authenticates_with_password_file` | Client password file authentication pending. |
| `tests/daemon.rs::client_respects_no_motd` | MOTD suppression feature incomplete. |
| `tests/daemon.rs::daemon_honors_bwlimit` | Bandwidth limiting not implemented. |
| `tests/filter_corpus.rs::filter_corpus_parity` | Filter corpus parity with upstream not yet validated. |
| `tests/filter_corpus.rs::perdir_sign_parity` | Per-directory signing parity pending. |
| `tests/cli.rs::progress_parity` | Progress output parity requires upstream comparison. |
| `tests/cli.rs::progress_parity_p` | Progress output parity requires upstream comparison. |
| `tests/cli.rs::default_umask_masks_permissions` | Umask handling under review. |
| `tests/no_implied_dirs.rs::preserves_symlinked_implied_dirs` | Symlinked implied directory behavior unfinished. |
| `tests/cli_flags.rs::blocking_io_nonblocking_by_default` | Blocking I/O flag behavior incomplete. |
| `tests/cli_flags.rs::blocking_io_flag_enables_blocking_mode` | Blocking I/O implementation pending. |
| `tests/remote_remote.rs::remote_remote_via_daemon_paths` | Remote-to-remote transfer through daemon not yet supported. |

