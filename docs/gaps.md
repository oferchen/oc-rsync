# Feature Gaps

This document summarizes parity status across major domains of `oc-rsync`.  Each
table lists notable features that are either implemented, only partially
completed, or still missing.  Entries link to the source and corresponding tests
when available. Do not exceed functionality of upstream at <https://rsync.samba.org> at this stage, prune unused features and/or unreachable code.

## Interop matrix scenarios

  - `base`: baseline transfer using [run_matrix.sh](../tests/interop/run_matrix.sh)
  - `delete`: `--delete` removes extraneous files
  - `delete_before`: `--delete-before` prunes destination prior to transfer
  - `delete_during`: `--delete-during` removes files mid-transfer
  - `delete_after`: `--delete-after` cleans up once transfer completes
  - `compress`: verifies `--compress` negotiation
  - `hard_links`: preserves hard links
  - `rsh`: remote shell invocation via `--rsh`
  - `drop_connection`: aborts transfer mid-stream
  - `vanished`: handles disappearing source files gracefully
  - `remote_remote`: third-party copy between two remotes
  - `append`: appends to existing destination files
  - `append_verify`: verifies appended data with checksums
  - `partial`: keeps partially transferred files
  - `inplace`: updates destination files in place
  - `resume`: resumes interrupted transfer with `--partial`
  - `progress`: shows incremental progress output
  - `resume_progress`: resumes with progress enabled
  - `progress2`: uses `--info=progress2` for aggregate progress
  - `resume_progress2`: resumes transfer with progress2 output

## Parser Parity
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Comprehensive flag parsing and help text parity | ✅ | [tests/cli.rs](../tests/cli.rs)<br>[tests/help_output.rs](../tests/help_output.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| Composite `--archive` flag expansion | ✅ | [tests/archive.rs](../tests/archive.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| Remote-only option parsing (`--remote-option`) | ✅ | [tests/interop/remote_option.rs](../tests/interop/remote_option.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |

Note: [tests/archive.rs](../tests/archive.rs) demonstrates the composite `--archive` flag expansion.

_Future contributors: update this section when adding or fixing CLI parser behaviors._


## Protocol
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Frame multiplexing and keep-alives | ✅ | [crates/protocol/tests/mux_demux.rs](../crates/protocol/tests/mux_demux.rs) | [crates/protocol/src/mux.rs](../crates/protocol/src/mux.rs) |
| Version negotiation | ✅ | [crates/protocol/tests/server.rs](../crates/protocol/tests/server.rs) | [crates/protocol/src/server.rs](../crates/protocol/src/server.rs) |
| Challenge-response authentication | ✅ | [crates/protocol/tests/auth.rs](../crates/protocol/tests/auth.rs) | [crates/protocol/src/server.rs](../crates/protocol/src/server.rs) |

## Exit Codes
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Standard exit code mapping | Implemented | [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs) | [crates/protocol/src/lib.rs](../crates/protocol/src/lib.rs) |
| Remote exit code propagation | Implemented | [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs) | [crates/protocol/src/demux.rs](../crates/protocol/src/demux.rs) |

## Checksums
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Rolling and strong MD4/MD5/SHA-1/xxh64/xxh3/xxh128 hashes | ✅ | [crates/checksums/tests/golden.rs](../crates/checksums/tests/golden.rs)<br>[crates/checksums/tests/rsync.rs](../crates/checksums/tests/rsync.rs) | [crates/checksums/src/lib.rs](../crates/checksums/src/lib.rs) |

## Compression
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| zstd and zlib codecs | Implemented | [crates/compress/tests/codecs.rs](../crates/compress/tests/codecs.rs) | [crates/compress/src/lib.rs](../crates/compress/src/lib.rs) |
| `--skip-compress` suffix handling | Implemented | [tests/skip_compress.rs](../tests/skip_compress.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| LZ4 codec (experimental) | Planned post-parity ([#873](https://github.com/oferchen/oc-rsync/pull/873)) | — | — |

## Filters
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Include/Exclude parser | Implemented | [crates/filters/tests/include_exclude.rs](../crates/filters/tests/include_exclude.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| `.rsync-filter` merge semantics | Implemented | [crates/filters/tests/merge.rs](../crates/filters/tests/merge.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Rule logging and statistics | Implemented | [crates/filters/tests/rule_stats.rs](../crates/filters/tests/rule_stats.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |

## File Selection
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Path-delta encoding with uid/gid tables | Implemented | [crates/engine/tests/flist.rs](../crates/engine/tests/flist.rs) | [crates/filelist/src/lib.rs](../crates/filelist/src/lib.rs) |
| Group ID preservation | Implemented | [crates/engine/tests/flist.rs](../crates/engine/tests/flist.rs) | [crates/filelist/src/lib.rs](../crates/filelist/src/lib.rs) |
| Extended attributes and ACL entries | Implemented | [crates/engine/tests/flist.rs](../crates/engine/tests/flist.rs) | [crates/filelist/src/lib.rs](../crates/filelist/src/lib.rs) |
| Batched filesystem traversal | Implemented | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/walk/src/lib.rs](../crates/walk/src/lib.rs) |
| Maximum file-size filtering | Implemented | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/walk/src/lib.rs](../crates/walk/src/lib.rs) |
| `--one-file-system` device boundary | Implemented | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/walk/src/lib.rs](../crates/walk/src/lib.rs) |

## Metadata Fidelity
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Permissions and ownership restoration | Implemented | [crates/meta/tests/chmod.rs](../crates/meta/tests/chmod.rs) | [crates/meta/src/unix.rs](../crates/meta/src/unix.rs) |
| `--fake-super` xattr fallback | Implemented | [crates/meta/tests/fake_super.rs](../crates/meta/tests/fake_super.rs) | [crates/meta/src/unix.rs](../crates/meta/src/unix.rs) |
| POSIX ACL preservation | Implemented | [crates/meta/tests/acl_roundtrip.rs](../crates/meta/tests/acl_roundtrip.rs) | [crates/meta/src/unix.rs](../crates/meta/src/unix.rs) |

## Transport
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| SSH stdio transport | ✅ | [crates/transport/tests/ssh_stdio.rs](../crates/transport/tests/ssh_stdio.rs) | [crates/transport/src/ssh.rs](../crates/transport/src/ssh.rs) |
| TCP transport with bandwidth limiting | ✅ | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) | [crates/transport/src/rate.rs](../crates/transport/src/rate.rs) |
| Extended socket options | ✅ | [crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs) | [crates/transport/src/tcp.rs](../crates/transport/src/tcp.rs) |

## Engine
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| In-place updates and resume | ✅ | [crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| Delete policies | ✅ | [crates/engine/tests/delete.rs](../crates/engine/tests/delete.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| `--read-batch` replay | ✅ | [crates/engine/tests/upstream_batch.rs](../crates/engine/tests/upstream_batch.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |

## Daemon Features
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Module parsing and secrets auth | Implemented | [tests/daemon_config.rs](../tests/daemon_config.rs)<br>[tests/daemon_auth.sh](../tests/daemon_auth.sh) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| IPv6 listener and rate limiting | Implemented | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Chroot and uid/gid dropping | Implemented | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| `rsyncd.conf` file parsing | Implemented | [tests/daemon_config.rs](../tests/daemon_config.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |

## Messages/Logging
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Custom out-format and log file messages | Implemented | [tests/out_format.rs](../tests/out_format.rs)<br>[tests/log_file.rs](../tests/log_file.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |
| System log integration (syslog/journald) | Implemented | [crates/logging/tests/syslog.rs](../crates/logging/tests/syslog.rs)<br>[crates/logging/tests/journald.rs](../crates/logging/tests/journald.rs)<br>[tests/daemon_syslog.rs](../tests/daemon_syslog.rs)<br>[tests/daemon_journald.rs](../tests/daemon_journald.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs)<br>[crates/cli/src/lib.rs](../crates/cli/src/lib.rs)<br>[crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Daemon MOTD/greeting messages | Implemented | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Info and debug flag routing | Implemented | [crates/logging/tests/info_flags.rs](../crates/logging/tests/info_flags.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |
| JSON and text formatters | Implemented | [crates/logging/tests/levels.rs](../crates/logging/tests/levels.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |

_Future contributors: update this section when adding or fixing message behaviors._

## CLI
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Comprehensive flag parsing via `clap` | ✅ | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| `--log-file-format` | ✅ | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| `--munge-links` option | ✅ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
### Outstanding Options

The following flags are parsed but lack verification against upstream `rsync`. Add interop tests to confirm parity.

- `--config`: add interop tests verifying parity. Tests: [tests/daemon_config.rs](../tests/daemon_config.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--copy-as`: requires root or CAP_CHOWN; add privileged interop tests. Tests: [tests/copy_as.rs](../tests/copy_as.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--dry-run`: add interop tests ensuring no file changes. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--early-input`: add interop tests verifying early argument handling. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--fake-super`: requires `xattr` feature; add interop tests. Tests: [tests/fake_super.rs](../tests/fake_super.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--fsync`: add interop tests verifying fsync semantics. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--fuzzy`: add interop tests for fuzzy matching. Tests: [tests/fuzzy.rs](../tests/fuzzy.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--groupmap`: requires root or CAP_CHOWN; add privileged interop tests. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--help`: add interop tests comparing help output. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--ignore-errors`: add interop tests verifying delete behavior. Tests: [tests/delete_policy.rs](../tests/delete_policy.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--ignore-missing-args`: add interop tests for missing arg handling. Tests: [tests/ignore_missing_args.rs](../tests/ignore_missing_args.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--ignore-times`: add interop tests for timestamp handling. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--info`: add interop tests for info flag routing. Tests: [crates/logging/tests/info_flags.rs](../crates/logging/tests/info_flags.rs)<br>[crates/cli/tests/logging_flags.rs](../crates/cli/tests/logging_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--max-size`: add interop tests validating size filtering. Tests: [tests/perf_limits.rs](../tests/perf_limits.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--min-size`: add interop tests validating size filtering. Tests: [tests/perf_limits.rs](../tests/perf_limits.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--modify-window`: add interop tests for close mtime handling. Tests: [tests/modify_window.rs](../tests/modify_window.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--old-args`: add interop tests for legacy arg protection. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--old-d`: add interop tests for legacy `--dirs`. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: —.
- `--old-dirs`: add interop tests for legacy directory handling. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--open-noatime`: add interop tests on platforms with `O_NOATIME`. Tests: [crates/engine/tests/open_noatime.rs](../crates/engine/tests/open_noatime.rs)<br>[tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs)<br>[crates/engine/src/lib.rs](../crates/engine/src/lib.rs).
- `--outbuf`: add interop tests verifying stdout buffering. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [bin/oc-rsync/src/main.rs](../bin/oc-rsync/src/main.rs).
- `--progress`: add interop tests for progress output. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `-P`: add interop tests verifying shorthand for `--partial --progress`. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--protocol`: add interop tests verifying protocol negotiation. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--relative`: add interop tests for path handling. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--secluded-args`: add interop tests verifying argument separation. Tests: [tests/secluded_args.rs](../tests/secluded_args.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--server`: add interop tests for protocol version and codec negotiation. Tests: [crates/protocol/tests/server.rs](../crates/protocol/tests/server.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--sockopts`: add interop tests for socket option handling. Tests: [tests/sockopts.rs](../tests/sockopts.rs)<br>[crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--specials`: add interop tests for special file support. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--stats`: add interop tests for statistics output. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--stop-after`: add interop tests for stop-after semantics. Tests: [tests/timeout.rs](../tests/timeout.rs). Source: [crates/cli/src/options.rs](../crates/cli/src/options.rs)<br>[crates/engine/src/lib.rs](../crates/engine/src/lib.rs).
- `--stop-at`: add interop tests for stop-at semantics. Tests: [tests/timeout.rs](../tests/timeout.rs). Source: [crates/cli/src/options.rs](../crates/cli/src/options.rs)<br>[crates/engine/src/lib.rs](../crates/engine/src/lib.rs).
- `--super`: add interop tests for super-user mode. Tests: [tests/cli.rs](../tests/cli.rs)<br>[crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--temp-dir`: add interop tests ensuring same-filesystem behavior. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--timeout`: add interop tests for idle and I/O timeouts. Tests: [tests/timeout.rs](../tests/timeout.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--trust-sender`: add interop tests confirming no-op behavior. Tests: [tests/cli_flags.rs](../tests/cli_flags.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--update`: add interop tests for `--update` semantics. Tests: [crates/engine/tests/update.rs](../crates/engine/tests/update.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--usermap`: requires root or CAP_CHOWN; add privileged interop tests. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--verbose`: add interop tests verifying verbosity levels. Tests: [tests/cli.rs](../tests/cli.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).
- `--xattrs`: requires `xattr` feature; add interop tests. Tests: [tests/local_sync_tree.rs](../tests/local_sync_tree.rs)<br>[tests/daemon_sync_attrs.rs](../tests/daemon_sync_attrs.rs). Source: [crates/cli/src/lib.rs](../crates/cli/src/lib.rs).

## Test Coverage
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Workspace coverage via `cargo llvm-cov` (≥95%) | Implemented | [reports/metrics.md](../reports/metrics.md) | [Makefile](../Makefile) |
| Coverage exclusions documented | Implemented | — | [coverage_exclusions.md](coverage_exclusions.md) |

