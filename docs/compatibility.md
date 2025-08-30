# Compatibility

This page summarizes the operating systems and interoperability scenarios that
have been exercised with `rsync-rs`. For a detailed status matrix see
[compat_matrix.md](compat_matrix.md).

## Tested platforms

| Operating system | Notes |
|------------------|-------|
| Linux | primary development and CI platform |
| Linux (armv7) | cross-compiled in CI |
| FreeBSD | cross-compiled in CI |
| macOS | builds and basic local sync verified |
| Windows | under active development; path and permission handling incomplete |

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

