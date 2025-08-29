# Security Policy

## Reporting a Vulnerability

Please report suspected vulnerabilities to [security@rsync.rs](mailto:security@rsync.rs). We aim to respond within three business days and will coordinate a fix prior to public disclosure.

## Supported Versions

| Version | Support lifetime |
| ------- | ---------------- |
| Latest release | 12 months from publication |
| Older releases | Unsupported |

## Threat Model & Mitigations

`rsync-rs` assumes untrusted peers and adversarial networks. The daemon and transport stack employ multiple layers of defense:

### Chroot jail and privilege drop
Daemon modules are served from a chrooted view of the filesystem and run as an unprivileged user and group (UID/GID 65534) to limit the impact of compromise. See [daemon documentation](docs/daemon.md#L34-L36) and the [privilege drop implementation](crates/cli/src/lib.rs#L1225-L1229).

### Path normalization & traversal prevention
All file paths are resolved relative to their declared roots. The sync engine strips prefixes before acting on any path, preventing `..` segments or symlink tricks from escaping the module directory. Example checks appear in [`delete_extraneous`](crates/engine/src/lib.rs#L820-L839) and [`sync`](crates/engine/src/lib.rs#L867-L879).

### UID/GID mapping controls
Ownership metadata is mapped by user and group name unless the caller opts into raw IDs with `--numeric-ids`. This ensures UID/GID translation can be enforced or bypassed deliberately. See [numeric ID handling](docs/daemon.md#L26-L31) and the [`numeric_ids` option](crates/engine/src/lib.rs#L744-L780).

### Network and I/O timeouts, resource caps
Idle peers are detected by the protocol demultiplexer, which errors when a channel exceeds its timeout. Transports expose socket read timeouts and optional bandwidth limits to prevent resource exhaustion. Relevant pieces include [Demux timeouts](crates/protocol/src/demux.rs#L12-L23), [timeout checks](crates/protocol/src/demux.rs#L78-L85), [`TcpTransport` read timeouts](crates/transport/src/tcp.rs#L25-L37), and [rate limiting](crates/transport/src/rate.rs#L6-L35).

### Input validation and auth handling
Clients may authenticate with a secrets file restricted to mode `0600`. Tokens are validated and unauthorized attempts are rejected before any module access. See [secrets-file authentication](docs/daemon.md#L15-L24) and the [token validation logic](crates/cli/src/lib.rs#L1242-L1279).

### Safe defaults
Security-sensitive features default to the safest possible settings. Weak algorithms require explicit opt-in, destructive actions such as recursive delete are disabled by default, and configuration is read only from trusted locations.

