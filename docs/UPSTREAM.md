# Upstream Compatibility

`oc-rsync` interoperates with classic `rsync` by negotiating a common protocol version during the handshake.
The following upstream protocol versions are recognized:

- 32
- 31
- 30
- 29

When connecting to a peer, the highest shared version from this list is selected. Versions earlier than 29 are not supported.
