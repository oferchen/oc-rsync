# Feature Gaps

All divergences from upstream `rsync` should be recorded here; the previous `docs/differences.md` file is intentionally empty.

This document summarizes parity status across major domains of `oc-rsync`.  Each
table lists notable features that are either implemented, only partially
completed, or still missing.  Entries link to the source and corresponding tests
when available. Do not exceed functionality of upstream at <https://rsync.samba.org> at this stage, prune unused features and/or unreachable code.

## Interop matrix scenarios

The interoperability matrix builds upstream `rsync 3.4.1` via
[tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh) and exercises real
transfers across the following scenarios:

  - `base`: baseline transfer using [tests/interop/run_matrix.sh](../tests/interop/run_matrix.sh)
  - `delete`: `--delete` removes extraneous files
  - `compress_zlib`: zlib negotiation using [codec_negotiation.rs](../tests/interop/codec_negotiation.rs)
  - `compress_zstd`: zstd negotiation using [codec_negotiation.rs](../tests/interop/codec_negotiation.rs)
  - `filters`: include/exclude and `.rsync-filter` rules via [filter_complex.rs](../tests/interop/filter_complex.rs)
  - `metadata`: ACL, xattr and permission preservation validated against [golden fixtures](../tests/interop/golden)
  - `partial`: `--partial` leaves resumable files in place as demonstrated in [resume.rs](../tests/resume.rs)
  - `resume`: interrupted transfers resume from partial files in [resume.rs](../tests/resume.rs)
  - `vanished`: vanished source files handled gracefully

## Parser Parity
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Comprehensive flag parsing and help text parity | ✅ | [tests/cli.rs](../tests/cli.rs)<br>[tests/help_output.rs](../tests/help_output.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| Composite `--archive` flag expansion | ✅ | [tests/archive.rs](../tests/archive.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| Remote-only option parsing (`--remote-option`) | ✅ | [tests/interop/remote_option.rs](../tests/interop/remote_option.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| `--version` output parity | ✅ | [tests/version_output.rs](../tests/version_output.rs) | [crates/cli/src/version.rs](../crates/cli/src/version.rs) |
| Null-delimited list parsing (`--from0`) | ✅ | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |

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
| Rolling and strong MD4/MD5/SHA-1 hashes | ✅ | [crates/checksums/tests/golden.rs](../crates/checksums/tests/golden.rs)<br>[crates/checksums/tests/rsync.rs](../crates/checksums/tests/rsync.rs) | [crates/checksums/src/lib.rs](../crates/checksums/src/lib.rs) |

## Compression
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| zstd and zlib codecs | Implemented | [crates/compress/tests/codecs.rs](../crates/compress/tests/codecs.rs) | [crates/compress/src/lib.rs](../crates/compress/src/lib.rs) |
| `--skip-compress` suffix handling | Implemented | [tests/skip_compress.rs](../tests/skip_compress.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| LZ4 codec | Planned post-parity ([#873](https://github.com/oferchen/oc-rsync/pull/873)); `liblz4-dev` no longer required for interop builds | — | — |

## Filters
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Include/Exclude parser | Implemented | [crates/filters/tests/include_exclude.rs](../crates/filters/tests/include_exclude.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| `.rsync-filter` merge semantics | Implemented | [crates/filters/tests/merge.rs](../crates/filters/tests/merge.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Null-delimited filter lists and merges (`--from0`) | Implemented | [crates/filters/tests/include_from.rs](../crates/filters/tests/include_from.rs)<br>[crates/filters/tests/from0_merges.rs](../crates/filters/tests/from0_merges.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Rule logging and statistics | Implemented | [crates/filters/tests/rule_stats.rs](../crates/filters/tests/rule_stats.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Additional rule modifiers | Implemented | [crates/filters/tests/rule_modifiers.rs](../crates/filters/tests/rule_modifiers.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| CVS ignore semantics (`--cvs-exclude`) | ✅ | [tests/cvs_exclude.rs](../tests/cvs_exclude.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Complex glob patterns | Implemented | [crates/filters/tests/advanced_globs.rs](../crates/filters/tests/advanced_globs.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| `--files-from` directory entries | Implemented | [crates/filters/tests/files_from.rs](../crates/filters/tests/files_from.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Directory boundary handling | Implemented | [tests/cli.rs](../tests/cli.rs) (`single_star_does_not_cross_directories`<br>`segment_star_does_not_cross_directories`) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |

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
| Windows metadata preservation | Implemented | [tests/windows.rs](../tests/windows.rs) | [crates/meta/src/windows.rs](../crates/meta/src/windows.rs) |

## Transport
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| SSH stdio transport | ✅ | [crates/transport/tests/ssh_stdio.rs](../crates/transport/tests/ssh_stdio.rs) | [crates/transport/src/ssh.rs](../crates/transport/src/ssh.rs) |
| TCP transport with bandwidth limiting | ✅ | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) | [crates/transport/src/rate.rs](../crates/transport/src/rate.rs) |
| Extended socket options | ✅ | [crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs) | [crates/transport/src/tcp.rs](../crates/transport/src/tcp.rs) |
| Connection error parity (SSH/daemon) | ✅ | [tests/interop/failure_cases.rs](../tests/interop/failure_cases.rs) | [crates/transport/src/ssh.rs](../crates/transport/src/ssh.rs)<br>[crates/transport/src/tcp.rs](../crates/transport/src/tcp.rs) |

## Engine
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| In-place updates and resume | ✅ | [crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| Delete policies | ✅ | [crates/engine/tests/delete.rs](../crates/engine/tests/delete.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| `--read-batch` replay | ✅ | [crates/engine/tests/upstream_batch.rs](../crates/engine/tests/upstream_batch.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| `--block-size` semantics | Partial | [tests/block_size.rs](../tests/block_size.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |

`--block-size` adjusts the delta algorithm's chunk size. To mirror upstream
behavior, it is typically combined with `--checksum` and `--no-whole-file` so
that only changed blocks are transferred.

```bash
$ oc-rsync --checksum --no-whole-file --block-size=4K --stats src/ dst/
Literal data: 4,096 bytes
```

The stats output shows that only a single 4 KiB block was sent.
**Pending:** Engine stats do not yet report literal byte counts accurately; see [#1522](https://github.com/oferchen/oc-rsync/issues/1522) and [tests/block_size.rs](../tests/block_size.rs).

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
| `--dry-run` prevents destination changes | ✅ | [tests/interop/dry_run.rs](../tests/interop/dry_run.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| Test-only `--dump-help-body` flag for help text verification | Internal | [tests/help_output.rs](../tests/help_output.rs) | [crates/cli/src/options.rs](../crates/cli/src/options.rs) |
### Outstanding Options

All CLI flags now have interop coverage verifying parser and message parity with upstream `rsync`. See [tests/interop/outstanding_flags.rs](../tests/interop/outstanding_flags.rs).

## Test Coverage
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Workspace coverage via `cargo llvm-cov` (≥95%) | Implemented | [reports/metrics.md](../reports/metrics.md) | [Makefile](../Makefile) |
| Coverage exclusions documented | Implemented | — | [coverage_exclusions.md](coverage_exclusions.md) |

