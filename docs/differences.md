# Differences

- **Transport edge cases**: SSH and daemon transports are early implementations and may diverge from classic `rsync` behavior.
  _Planned resolution_: expand interoperability tests and align protocol handling with upstream.

- **Windows support**: path and permission handling on Windows is incomplete.
  _Planned resolution_: continue cross-platform development until parity is reached.

- **Extended attributes and ACLs**: upstream `rsync` handles these by default, but `oc-rsync` requires building with `xattr` and `acl` feature gates.
  _Planned resolution_: polish feature gating and ensure consistent behavior across platforms.
