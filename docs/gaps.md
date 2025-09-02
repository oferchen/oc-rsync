# Feature Gaps

This document summarizes parity status across major domains of `oc-rsync`.  Each
table lists notable features that are either implemented, only partially
completed, or still missing.  Entries link to the source and corresponding tests
when available.

## Protocol
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Frame multiplexing and keep-alives | ✅ | [crates/protocol/tests/mux_demux.rs](../crates/protocol/tests/mux_demux.rs) | [crates/protocol/src/mux.rs](../crates/protocol/src/mux.rs) |
| Version negotiation | ✅ | [crates/protocol/tests/server.rs](../crates/protocol/tests/server.rs) | [crates/protocol/src/server.rs](../crates/protocol/src/server.rs) |
| Exit code propagation | ⚠️ | [crates/protocol/tests/exit_codes.rs](../crates/protocol/tests/exit_codes.rs) | [crates/protocol/src/lib.rs](../crates/protocol/src/lib.rs) |
| Challenge-response authentication | ✅ | [crates/protocol/tests/auth.rs](../crates/protocol/tests/auth.rs) | [crates/protocol/src/server.rs](../crates/protocol/src/server.rs) |

## Checksums
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Rolling and strong MD4/MD5/SHA-1 hashes | ✅ | [crates/checksums/tests/golden.rs](../crates/checksums/tests/golden.rs)<br>[crates/checksums/tests/rsync.rs](../crates/checksums/tests/rsync.rs) | [crates/checksums/src/lib.rs](../crates/checksums/src/lib.rs) |

## Compression
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| zstd, zlibx and zlib codecs | ✅ | [crates/compress/tests/codecs.rs](../crates/compress/tests/codecs.rs) | [crates/compress/src/lib.rs](../crates/compress/src/lib.rs) |
| `--skip-compress` suffix handling | ✅ | [tests/skip_compress.rs](../tests/skip_compress.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| Additional codecs (e.g. lzo, lz4) | ❌ | — | — |

## Filters
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Include/Exclude parser | ✅ | [crates/filters/tests/include_exclude.rs](../crates/filters/tests/include_exclude.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| `.rsync-filter` merge semantics | ✅ | [crates/filters/tests/merge.rs](../crates/filters/tests/merge.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |
| Rule logging and statistics | ✅ | [crates/filters/tests/rule_stats.rs](../crates/filters/tests/rule_stats.rs) | [crates/filters/src/lib.rs](../crates/filters/src/lib.rs) |

## File List
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Path-delta encoding with uid/gid tables | ✅ | [crates/engine/tests/flist.rs](../crates/engine/tests/flist.rs) | [crates/filelist/src/lib.rs](../crates/filelist/src/lib.rs) |
| Group ID preservation | ✅ | [crates/engine/tests/flist.rs](../crates/engine/tests/flist.rs) | [crates/filelist/src/lib.rs](../crates/filelist/src/lib.rs) |
| Extended attributes and ACL entries | ✅ | [crates/engine/tests/flist.rs](../crates/engine/tests/flist.rs) | [crates/filelist/src/lib.rs](../crates/filelist/src/lib.rs) |

## Walk
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Batched filesystem traversal | ✅ | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/walk/src/lib.rs](../crates/walk/src/lib.rs) |
| Maximum file-size filtering | ✅ | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/walk/src/lib.rs](../crates/walk/src/lib.rs) |
| `--one-file-system` device boundary | ✅ | [crates/walk/tests/walk.rs](../crates/walk/tests/walk.rs) | [crates/walk/src/lib.rs](../crates/walk/src/lib.rs) |

## Metadata
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Permissions and ownership restoration | ✅ | [crates/meta/tests/chmod.rs](../crates/meta/tests/chmod.rs) | [crates/meta/src/unix.rs](../crates/meta/src/unix.rs) |
| `--fake-super` xattr fallback | ✅ | [crates/meta/tests/fake_super.rs](../crates/meta/tests/fake_super.rs) | [crates/meta/src/unix.rs](../crates/meta/src/unix.rs) |
| POSIX ACL preservation | ❌ | — | — |

## Transport
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| SSH stdio transport | ✅ | [crates/transport/tests/ssh_stdio.rs](../crates/transport/tests/ssh_stdio.rs) | [crates/transport/src/ssh.rs](../crates/transport/src/ssh.rs) |
| TCP transport with bandwidth limiting | ✅ | [crates/transport/tests/bwlimit.rs](../crates/transport/tests/bwlimit.rs) | [crates/transport/src/rate.rs](../crates/transport/src/rate.rs) |
| Extended socket options | ⚠️ | [crates/transport/tests/sockopts.rs](../crates/transport/tests/sockopts.rs) | [crates/transport/src/tcp.rs](../crates/transport/src/tcp.rs) |
| Connection retry/backoff | ❌ | — | — |

## Engine
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| In-place updates and resume | ✅ | [crates/engine/tests/resume.rs](../crates/engine/tests/resume.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| Delete policies | ✅ | [crates/engine/tests/delete.rs](../crates/engine/tests/delete.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |
| `--read-batch` replay | ✅ | [crates/engine/tests/upstream_batch.rs](../crates/engine/tests/upstream_batch.rs) | [crates/engine/src/lib.rs](../crates/engine/src/lib.rs) |

## Daemon
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Module parsing and secrets auth | ✅ | [tests/daemon_config.rs](../tests/daemon_config.rs)<br>[tests/daemon_auth.sh](../tests/daemon_auth.sh) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| IPv6 listener and rate limiting | ✅ | [tests/daemon.rs](../tests/daemon.rs) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| Chroot and uid/gid dropping | ⚠️ | [tests/daemon_features.sh](../tests/daemon_features.sh) | [crates/daemon/src/lib.rs](../crates/daemon/src/lib.rs) |
| `rsyncd.conf` file parsing | ❌ | — | — |

## CLI
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Comprehensive flag parsing via `clap` | ✅ | [tests/cli.rs](../tests/cli.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| `--log-file-format` (limited subset) | ⚠️ | [tests/log_file.rs](../tests/log_file.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |
| `--munge-links` option | ✅ | [tests/symlink_resolution.rs](../tests/symlink_resolution.rs) | [crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |

## Logging
| Feature | Status | Tests | Source |
| --- | --- | --- | --- |
| Info and debug flag routing | ✅ | [crates/logging/tests/info_flags.rs](../crates/logging/tests/info_flags.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |
| JSON and text formatters | ✅ | [crates/logging/tests/levels.rs](../crates/logging/tests/levels.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs) |
| System log integration | ✅ | [crates/logging/tests/syslog.rs](../crates/logging/tests/syslog.rs)<br>[crates/logging/tests/journald.rs](../crates/logging/tests/journald.rs) | [crates/logging/src/lib.rs](../crates/logging/src/lib.rs)<br>[crates/cli/src/lib.rs](../crates/cli/src/lib.rs) |

