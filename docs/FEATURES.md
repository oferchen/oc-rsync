# Feature Overview

This document tracks the major capabilities planned for the project.

| Feature | Status | Milestone |
| --- | --- | --- |
| Directory traversal and checksum generation | Implemented | [M1—Bootstrap][M1] |
| Local sync via CLI | Implemented | [M1—Bootstrap][M1] |
| Delta-transfer engine | In progress | [M2—Transfer Engine][M2] |
| Resume/partial transfers | Planned | [M2—Transfer Engine][M2] |
| Network transport (SSH/TCP) | In progress | [M3—Networking][M3] |
| Permissions (ownership, timestamps, modes) | In progress | [M4—CLI Parity][M4] |
| Symlink preservation | In progress | [M4—CLI Parity][M4] |
| Hardlink support | Planned | [M7—Stabilization][M7] |
| Sparse file handling | Planned | [M7—Stabilization][M7] |
| Extended attributes (xattrs) & ACLs | Implemented | [M7—Stabilization][M7] |
| Include/exclude filters | In progress | [M5—Filters & Compression][M5] |
| Compression negotiation | In progress | [M5—Filters & Compression][M5] |
| Human-readable stats output (`--human-readable`) | Implemented | [M4—CLI Parity][M4] |

[M1]: https://github.com/rsync-rs/rsync_rs/milestone/1
[M2]: https://github.com/rsync-rs/rsync_rs/milestone/2
[M3]: https://github.com/rsync-rs/rsync_rs/milestone/3
[M4]: https://github.com/rsync-rs/rsync_rs/milestone/4
[M5]: https://github.com/rsync-rs/rsync_rs/milestone/5
[M7]: https://github.com/rsync-rs/rsync_rs/milestone/7
