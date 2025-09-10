# Feature Gaps

All divergences from upstream `rsync` should be recorded here; the previous `docs/differences.md` file is intentionally empty.

This document summarizes parity status across major domains of `oc-rsync`. Each table lists notable features that are implemented, partially complete, missing, or divergent from upstream behavior. Entries link to the source and corresponding tests when available. Do not exceed functionality of upstream at <https://rsync.samba.org> at this stage, prune unused features and/or unreachable code.

## CLI & Parser
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Comprehensive flag parsing and help text parity | Implemented | [tests/cli_flags.rs](../tests/cli_flags.rs)<br>[crates/cli/tests/help.rs](../crates/cli/tests/help.rs) | [crates/cli/src/argparse.rs](../crates/cli/src/argparse.rs) |
| Composite `--archive` flag expansion | Implemented | [tests/archive.rs](../tests/archive.rs) | [crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| Remote-only option parsing (`--remote-option`) | Implemented | [tests/interop/remote_option.rs](../tests/interop/remote_option.rs) | [crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| `--version` output parity | Implemented | [tests/version_output.rs](../tests/version_output.rs) | [crates/cli/src/version.rs](../crates/cli/src/version.rs) |
| Null-delimited list parsing (`--from0`) | Implemented | [tests/files_from.rs](../tests/files_from.rs) | [crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| `--log-file-format` | Implemented | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| `--munge-links` option | Implemented | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/argparse.rs](../crates/cli/src/argparse.rs) |
| `--dry-run` prevents destination changes | Implemented | [tests/interop/dry_run.rs](../tests/interop/dry_run.rs) | [crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| Test-only `--dump-help-body` flag for help text verification | Implemented | [crates/cli/tests/help.rs](../crates/cli/tests/help.rs) | [crates/cli/src/argparse.rs](../crates/cli/src/argparse.rs) |

## Protocol
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Frame multiplexing and keep-alives | Implemented | [crates/protocol/tests/mux_demux.rs](../crates/protocol/tests/mux_demux.rs) | [crates/protocol/src/mux.rs](../crates/protocol/src/mux.rs) |
| Version negotiation | Implemented | [crates/protocol/tests/server.rs](../crates/protocol/tests/server.rs) | [crates/protocol/src/server.rs](../crates/protocol/src/server.rs) |
| Challenge-response authentication | Implemented | [crates/protocol/tests/auth.rs](../crates/protocol/tests/auth.rs) | [crates/protocol/src/server.rs](../crates/protocol/src/server.rs) |

## Filters
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Include/Exclude parser | Implemented | [crates/filters/tests/include_exclude.rs](../crates/filters/tests/include_exclude.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| `.rsync-filter` merge semantics | Implemented | [crates/filters/tests/merge.rs](../crates/filters/tests/merge.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Null-delimited filter lists and merges (`--from0`) | Implemented | [crates/filters/tests/include_from.rs](../crates/filters/tests/include_from.rs)<br>[crates/filters/tests/from0_merges.rs](../crates/filters/tests/from0_merges.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Rule logging and statistics | Implemented | [crates/filters/tests/rule_stats.rs](../crates/filters/tests/rule_stats.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Additional rule modifiers | Implemented | [crates/filters/tests/rule_modifiers.rs](../crates/filters/tests/rule_modifiers.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| CVS ignore semantics (`--cvs-exclude`) | Implemented | [tests/cvs_exclude.rs](../tests/cvs_exclude.rs)<br>[crates/filters/tests/cvs_rules.rs](../crates/filters/tests/cvs_rules.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Complex glob patterns | Implemented | [crates/filters/tests/advanced_globs.rs](../crates/filters/tests/advanced_globs.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| `--files-from` directory entries | Implemented | [crates/filters/tests/files_from.rs](../crates/filters/tests/files_from.rs)<br>[tests/files_from_dirs.rs](../tests/files_from_dirs.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs)<br>[crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| Directory boundary handling | Implemented | [tests/misc.rs](../tests/misc.rs) (`single_star_does_not_cross_directories` / `segment_star_does_not_cross_directories`) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |

## Metadata
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Permissions and ownership restoration | Implemented | [crates/meta/tests/chmod.rs](../crates/meta/tests/chmod.rs) | [crates/meta/src/unix/mod.rs](../crates/meta/src/unix/mod.rs) |
| `--fake-super` xattr fallback | Implemented | [crates/meta/tests/fake_super.rs](../crates/meta/tests/fake_super.rs) | [crates/meta/src/unix/mod.rs](../crates/meta/src/unix/mod.rs) |
| POSIX ACL preservation | Implemented | [crates/meta/tests/acl_roundtrip.rs](../crates/meta/tests/acl_roundtrip.rs) | [crates/meta/src/unix/mod.rs](../crates/meta/src/unix/mod.rs) |
| Hard link detection and recreation | Implemented | [tests/hard_links.rs](../tests/hard_links.rs)<br>[crates/engine/tests/links.rs](../crates/engine/tests/links.rs) | [crates/meta/src/lib.rs](../crates/meta/src/lib.rs) |
| Windows metadata preservation | Implemented | [tests/windows.rs](../tests/windows.rs) | [crates/meta/src/windows/mod.rs](../crates/meta/src/windows/mod.rs) |

## Compression
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| zstd and zlib codecs | Implemented | [crates/compress/tests/codecs.rs](../crates/compress/tests/codecs.rs) | [crates/compress/src/mod.rs](../crates/compress/src/mod.rs) |
| `--skip-compress` suffix handling | Implemented | [tests/skip_compress.rs](../tests/skip_compress.rs) | [crates/cli/src/client.rs](../crates/cli/src/client.rs) |
| LZ4 codec | Intentionally out of scope | — | — |

## Daemon
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Module parsing and secrets auth | Implemented | [tests/daemon_config.rs](../tests/daemon_config.rs)<br>[tests/daemon_auth.sh](../tests/daemon_auth.sh) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| IPv6 listener and rate limiting | Implemented | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Chroot and uid/gid dropping | Implemented | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| `rsyncd.conf` file parsing | Implemented | [tests/daemon_config.rs](../tests/daemon_config.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |

## Messages
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Custom out-format and log file messages | Implemented | [tests/out_format.rs](../tests/out_format.rs)<br>[tests/log_file.rs](../tests/log_file.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |
| System log integration (syslog/journald) | Implemented | [crates/logging/tests/syslog.rs](../crates/logging/tests/syslog.rs)<br>[crates/logging/tests/journald.rs](../crates/logging/tests/journald.rs)<br>[tests/daemon_syslog.rs](../tests/daemon_syslog.rs)<br>[tests/daemon_journald.rs](../tests/daemon_journald.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs)<br>[crates/cli/src/argparse.rs](../crates/cli/src/argparse.rs)<br>[crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Daemon MOTD/greeting messages | Implemented | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Info and debug flag routing | Implemented | [crates/logging/tests/info_flags.rs](../crates/logging/tests/info_flags.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |
| JSON and text formatters | Implemented | [crates/logging/tests/levels.rs](../crates/logging/tests/levels.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |

## Exit Codes
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Standard exit code mapping | Implemented | [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs) | [crates/protocol/src/lib.rs](../crates/protocol/src/lib.rs) |
| Remote exit code propagation | Implemented | [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs) | [crates/protocol/src/demux.rs](../crates/protocol/src/demux.rs) |

## Testing
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Interoperability matrix (base, delete, compress_zlib, compress_zstd, filters, metadata, partial, resume, vanished) | Implemented | [scripts/interop/run.sh](../scripts/interop/run.sh)<br>[scripts/interop/validate.sh](../scripts/interop/validate.sh) | [scripts/interop/run.sh](../scripts/interop/run.sh)<br>[scripts/interop/validate.sh](../scripts/interop/validate.sh) |
| Workspace coverage via `cargo llvm-cov` (≥95%) | Implemented | [reports/metrics.md](../reports/metrics.md) | [Makefile](../Makefile) |
| Coverage exclusions documented | Implemented | — | [coverage_exclusions.md](coverage_exclusions.md) |
