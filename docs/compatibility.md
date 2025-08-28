# Compatibility

This page summarizes the operating systems and interoperability scenarios that
have been exercised with `rsync-rs`. For a detailed status matrix see
[compat_matrix.md](compat_matrix.md).

## Tested platforms

| Operating system | Notes |
|------------------|-------|
| Linux | primary development and CI platform |
| macOS | builds and basic local sync verified |
| Windows | under active development; path and permission handling incomplete |

## Interoperability caveats

* Remote transports and daemon mode are early stage and may diverge from
  classic `rsync` behavior.
* Many `rsync` features such as filters, hard links, xattrs, and ACLs are not
  yet implemented.
* `--modern` mode requires peers to understand zstd compression and BLAKE3
  checksums.
* Filesystem differences (case sensitivity, permissions) across platforms may
  lead to subtle inconsistencies.

