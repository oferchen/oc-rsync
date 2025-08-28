# Security Policy

## Reporting a Vulnerability

Please report suspected vulnerabilities to [security@rsync.rs](mailto:security@rsync.rs). We aim to respond within three business days and will coordinate a fix prior to public disclosure.

## Supported Versions

| Version | Support lifetime |
| ------- | ---------------- |
| Latest release | 12 months from publication |
| Older releases | Unsupported |

## Privilege Drop

`rsync-rs` is expected to operate with least privilege. When started with elevated rights it should drop to an unprivileged user as soon as possible and avoid regaining privileges. Deployments should run the service under a dedicated account with minimal filesystem and network permissions.

## Path Normalization

The project defends against path traversal by normalizing all input paths before use. Components reject paths containing `..`, symlink loops, or platform-specific quirks that could escape the intended root.

## Safe Defaults

Security-sensitive options default to their safest settings. Network endpoints require explicit opt-in for weak algorithms, and destructive operations such as recursive delete are disabled unless the user opts in. Configuration files are loaded only from trusted locations.

