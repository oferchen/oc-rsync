# Compatibility

This page summarizes the operating systems and interoperability scenarios that
have been exercised with `oc-rsync`. For a detailed status matrix see
[compat_matrix.md](compat_matrix.md). Cross-platform CI runs targeted tests on
Linux, macOS, and Windows.

## Protocol versions

`oc-rsync` interoperates with classic `rsync` using protocol versions 27
through 32.

## Tested platforms

| Operating system | Notes |
|------------------|-------|
| Linux | primary development and CI platform |
| Linux (arm64) | native CI runner |
| Linux (armv7) | cross-compiled in CI |
| FreeBSD | cross-compiled in CI (no tests) |
| macOS | CI runner executes targeted tests |
| Windows | CI runner executes targeted tests; path and permission handling incomplete |

## Supported Protocol Versions

| Version | Tests/Fixtures |
|---------|----------------|
| 27 | [proto-27 wire log](../tests/interop/wire/proto-27.log) |
| 28 | [proto-28 wire log](../tests/interop/wire/proto-28.log) |
| 29 | [version negotiation test](../crates/protocol/tests/protocol.rs#L40-L45) |
| 30 | [protocol override test](../crates/cli/src/lib.rs#L1958-L2030) |
| 31 | [server handshake test](../crates/protocol/tests/server.rs#L1-L80) |
| 32 | [rsync 3.3.0 transcript](../tests/interop/wire/rsync-3.3.0.log) |

## Interoperability caveats

* SSH and daemon transports are functional but still early implementations and
  may diverge from classic `rsync` behavior.
* Filters, sparse files, and compression work across transports. Hard links are
  not yet supported.
* Extended attributes and ACLs are available only when built with the `xattr`
  and `acl` feature gates and have not been widely exercised.
* Filesystem differences (case sensitivity, permissions) across platforms may
  lead to subtle inconsistencies.

