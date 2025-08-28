# Compatibility Matrix

## Platforms

| Platform | Status |
|----------|--------|
| Linux    | ✅ Planned full support |
| macOS    | ✅ Planned full support |
| Windows  | ⚠️ Initial work pending |

## Sync Modes

| Mode            | Status | Notes |
|-----------------|--------|-------|
| Local → Local   | ✅ Basic directory sync |
| Remote          | ❌ Not yet implemented |

## rsync Interoperability

| rsync Version | Transport | Filters | Hardlinks | Sparse | xattrs | ACLs | zlib | zstd |
|---------------|-----------|---------|-----------|--------|--------|------|------|------|
| 3.1.x         | SSH       | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 3.1.x         | rsync://  | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 3.2.x         | SSH       | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 3.2.x         | rsync://  | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

This matrix will be kept up to date by automated interoperability tests as
additional transports and feature flags are implemented.

Additional environments and modes may be evaluated in the future.
