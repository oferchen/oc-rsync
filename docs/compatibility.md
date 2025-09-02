# Compatibility

This page summarizes the operating systems and interoperability scenarios that
have been exercised with `oc-rsync`. Cross-platform tests are now integrated
into CI, and their results populate the detailed matrix in
[compat_matrix.md](compat_matrix.md). Known behavioral differences from classic
`rsync` are tracked in [differences.md](differences.md).

## Protocol versions

`oc-rsync` interoperates with classic `rsync` using protocol versions 29
through 32.

## Tested platforms

| Operating system | Notes |
|------------------|-------|
| Linux | primary development and CI platform |
| Linux (arm64) | native CI runner |
| Linux (armv7) | cross-compiled in CI |
| FreeBSD | cross-compiled in CI |
| macOS | builds and basic local sync verified |
| Windows | under active development; path and permission handling incomplete |

## Cross-platform tests

Automated cross-platform tests run in CI and validate transfers between Linux,
FreeBSD, macOS, and Windows. The [compat_matrix.md](compat_matrix.md) page is
kept up to date from their results.

## Supported Protocol Versions

| Version | Tests/Fixtures |
|---------|----------------|
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

