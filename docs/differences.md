# Differences

- **Transport edge cases**: SSH and daemon transports are early implementations and may diverge from classic `rsync` behavior.
  _Planned resolution_: expand interoperability tests and align protocol handling with upstream.

- **Windows support**: path and permission handling on Windows is incomplete.
  _Planned resolution_: continue cross-platform development until parity is reached.
