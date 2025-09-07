# Compatibility

This page summarizes the operating systems and interoperability scenarios that
have been exercised with `oc-rsync`. Cross-platform tests are now integrated
into CI, and their results populate the detailed matrix in
[compat_matrix.md](compat_matrix.md). Known behavioral differences from classic
`rsync` are tracked in [gaps.md](gaps.md).

## Protocol versions

`oc-rsync` interoperates with classic `rsync` using protocol versions 30
through 32, corresponding to upstream releases 3.0.9, 3.1.3, and 3.4.1.

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
| 30 | [rsync 3.0.9 transcript](../tests/interop/wire/rsync-3.0.9.log) |
| 31 | [rsync 3.1.3 transcript](../tests/interop/wire/rsync-3.1.3.log) |
| 32 | [rsync 3.4.1 transcript](../tests/interop/wire/rsync-3.4.1.log) |

## Interoperability caveats

* SSH and daemon transports are functional but still early implementations and
  may diverge from classic `rsync` behavior.
* Filters, sparse files, compression, and hard links work across transports.
* Extended attributes and ACLs are available only when built with the `xattr`
  and `acl` feature gates and are exercised by round-trip tests.
* Filesystem differences (case sensitivity, permissions) across platforms may
  lead to subtle inconsistencies.

