# Compatibility

This page summarizes the operating systems and interoperability scenarios that
have been exercised with `oc-rsync`. For a detailed status matrix see
[compat_matrix.md](compat_matrix.md).

## Protocol versions

`oc-rsync` interoperates with classic `rsync` using protocol versions 27
through 32 and negotiates version 73 when both peers enable modern mode.

## Tested platforms

| Operating system | Notes |
|------------------|-------|
| Linux | primary development and CI platform |
| Linux (arm64) | native CI runner |
| Linux (armv7) | cross-compiled in CI |
| FreeBSD | cross-compiled in CI |
| macOS | builds and basic local sync verified |
| Windows | under active development; path and permission handling incomplete |

## Supported Protocol Versions

| Version | Tests/Fixtures |
|---------|----------------|
| 27 | [proto-27 wire log](../tests/interop/wire/proto-27.log) |
| 28 | [proto-28 wire log](../tests/interop/wire/proto-28.log) |
| 29 | [version negotiation test](../crates/protocol/tests/protocol.rs#L40-L45) |
| 30 | [protocol override test](../crates/cli/src/lib.rs#L1958-L2030) |
| 31 | [server handshake test](../tests/server.rs#L34-L85) |
| 32 | [rsync 3.3.0 transcript](../tests/interop/wire/rsync-3.3.0.log) |
| 73 (modern) | [modern negotiation test](../tests/interop/modern.rs#L7-L10) |

## Interoperability caveats

* SSH and daemon transports are functional but still early implementations and
  may diverge from classic `rsync` behavior.
* Filters, sparse files, and compression work across transports. Hard links are
  not yet supported.
* Extended attributes and ACLs are available only when built with the `xattr`
  and `acl` feature gates and have not been widely exercised.
* `--modern` mode requires peers to understand zstd compression and BLAKE3 and is only available when built with the `blake3` feature
  checksums.
* Filesystem differences (case sensitivity, permissions) across platforms may
  lead to subtle inconsistencies.

