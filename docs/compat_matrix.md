# Compatibility Matrix

## Platforms

| Platform | Status |
|----------|--------|
| Linux    | ✅ Planned full support |
| macOS    | ✅ Planned full support |
| Windows  | ⚠️ Initial work pending |

## Sync Modes

| Mode                     | Status | Notes |
|--------------------------|--------|-------|
| Local → Local            | ✅ Basic directory sync |
| Local → Remote (SSH)     | ⚠️ Early interoperability |
| Local → Remote (daemon)  | ⚠️ Early interoperability |
| Remote → Remote          | ❌ Not yet implemented |

## Remote Feature Coverage

| Transport | Filters | Hardlinks | Sparse | xattrs | ACLs | zlib | zstd |
|-----------|---------|-----------|--------|--------|------|------|------|
| SSH       | ✅ | ❌ | ✅ | ✅* | ✅* | ✅ | ✅ |
| rsync://  | ✅ | ❌ | ✅ | ✅* | ✅* | ✅ | ✅ |

This matrix will be kept up to date by automated interoperability tests as
additional transports and feature flags are implemented.

* xattrs and ACLs require the corresponding `xattr` and `acl` feature gates.

Additional environments and modes may be evaluated in the future.
