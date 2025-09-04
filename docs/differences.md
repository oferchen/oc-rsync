# Differences

- **Transport edge cases**: SSH and daemon transports continue to mature and match classic `rsync` in common scenarios, though rare deviations may still surface.
  _Planned resolution_: expand interoperability tests to catch and fix remaining edge cases.

- **Windows support**: Windows builds are available, but path and permission handling still trail Unix platforms.
  _Planned resolution_: continue cross-platform development until parity is reached.
