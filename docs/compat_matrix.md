# Compatibility Matrix

## Platforms

| Platform | Status |
|----------|--------|
| Linux    | ✅ Full support |
| macOS    | ✅ Full support |
| Windows  | ⚠️ SSH transport unavailable |

## Sync Modes

| Mode                     | Status | Notes |
|--------------------------|--------|-------|
| Local → Local            | ✅ Full support | Parity with classic rsync |
| Local → Remote (SSH)     | ✅ Interoperates with classic rsync | Hardlinks pending |
| Local → Remote (daemon)  | ✅ Interoperates with classic rsync | Hardlinks pending |
| Remote → Remote          | ❌ Not yet implemented | — |

## Remote Feature Coverage

| Transport | Filters | Hardlinks | Sparse | xattrs | ACLs | zlib | zstd |
|-----------|---------|-----------|--------|--------|------|------|------|
| SSH       | ✅ | ❌ | ✅ | ✅* | ✅* | ✅ | ✅ |
| rsync://  | ✅ | ❌ | ✅ | ✅* | ✅* | ✅ | ✅ |

This matrix will be kept up to date by automated interoperability tests as
additional transports and feature flags are implemented.

* xattrs and ACLs require the corresponding `xattr` and `acl` feature gates.

Additional environments and modes may be evaluated in the future.
