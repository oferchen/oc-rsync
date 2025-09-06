# FLAKES

This file documents tests marked `#[ignore]` and why they remain excluded from automated runs.

| Test | Reason |
| ---- | ------ |
| `tests/packaging.rs::service_unit_matches_spec` | Service unit packaging checks require systemd packaging context and are not essential for core sync behavior. |
| `tests/daemon_config.rs::daemon_config_motd_suppression` | MOTD suppression support incomplete. |
| `tests/filter_corpus.rs::filter_corpus_parity` | Filter corpus parity with upstream not yet validated. |
| `tests/filter_corpus.rs::perdir_sign_parity` | Per-directory signing parity pending. |
| `tests/cli.rs::default_umask_masks_permissions` | Umask handling under review. |
| `tests/no_implied_dirs.rs::preserves_symlinked_implied_dirs` | Symlinked implied directory behavior unfinished. |
| `tests/remote_remote.rs::remote_remote_via_daemon_paths` | Remote-to-remote transfer through daemon not yet supported. |
