# Upstream Compatibility

`oc-rsync` interoperates with classic `rsync` by negotiating a common protocol version during the handshake.
The following upstream protocol versions are recognized:

- 32
- 31
- 30
- 29

When connecting to a peer, the highest shared version from this list is selected. Versions earlier than 29 are not supported.

## Version constants

```
UPSTREAM_VERSION=3.4.1
SUPPORTED_PROTOCOLS=[32,31,30,29]
```

The build scripts for `oc-rsync` and `oc-rsyncd` embed these values into the binaries by exporting them as compile-time environment variables (for example `cargo:rustc-env=RSYNC_UPSTREAM_VER=…`) in `build.rs`.
