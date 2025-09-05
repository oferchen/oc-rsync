# AGENTS.md

## Purpose
This document describes the agents, their responsibilities, and how they collaborate to achieve a full-featured, production-grade, pure-Rust reimplementation of **rsync** (protocol v32), maintaining full interoperability with upstream rsync. Each agent operates as a dedicated unit of functionality, designed to isolate complexity, maximize maintainability, and enforce clean code principles. The orchestration of these agents ensures **feature parity** with upstream `rsync`. Future enhancements may be explored only after parity is achieved.

---

## Agent Overview

### 1. **ProtocolAgent**
- **Role**: Implements the rsync v32 protocol, including message framing, multiplexing, negotiation, keep-alives, error propagation, and exit codes.
- **Responsibilities**:
  - Encode/decode frames (control, file list, data, checksums).
  - Manage sender/receiver protocol phases as a finite state machine.
  - Enforce compatibility with stock rsync across SSH and rsync:// transports.
  - Provide hooks for fuzzing and golden test replay.
- **Design Pattern**: **State Machine** for lifecycle phases, **Builder** for protocol negotiation settings.

---

### 2. **ChecksumAgent**
- **Role**: Calculates rolling and strong checksums for delta transfer and data integrity.
- **Responsibilities**:
  - Provide legacy algorithms (MD4, MD5, SHA1) for compatibility.
  - Additional algorithms may be considered once interoperability parity is complete.
  - Match rsync’s block-size heuristics and checksum-seed semantics.
  - Strategy pattern to select checksum at runtime, negotiated during protocol setup.
- **Design Pattern**: **Strategy** pattern for checksum selection.

---

### 3. **CompressionAgent**
- **Role**: Handles data compression and decompression streams.
- **Responsibilities**:
   - Support zlib and zstd (compatibility-first).
  - Further compression options may be considered after parity with upstream is reached.
  - Parse and apply `--compress-choice`, `--compress-level`, and `--skip-compress` arguments.
  - Maintain throughput and low latency for large trees.
- **Design Pattern**: **Strategy** pattern for algorithm choice; **Pipeline** integration with ProtocolAgent.

---

### 4. **FilterAgent**
- **Role**: Implements rsync’s filter engine (include/exclude rules).
- **Responsibilities**:
  - Parse and apply `--exclude`, `--include`, `--filter`, `--cvs-exclude`.
  - Merge per-directory `.rsync-filter` rules correctly.
  - Enforce order-dependent evaluation semantics identical to upstream rsync.
  - Provide fuzzing targets for parser correctness and property tests for precedence.
- **Design Pattern**: **Interpreter** pattern for filter parsing and evaluation.

---

### 5. **FileListAgent**
- **Role**: Builds and interprets the file list (flist).
- **Responsibilities**:
  - Stream file list generation and consumption.
  - Encode delta-encoded paths, uid/gid tables, and per-file metadata.
  - Optimize memory for very large directory trees (millions of entries).
  - Provide round-trip testing to guarantee correctness.
- **Design Pattern**: **Pipeline** stage integrated with WalkAgent and ProtocolAgent.

---

### 6. **WalkAgent**
- **Role**: Traverses filesystem trees to discover files, directories, and metadata.
- **Responsibilities**:
  - Perform efficient filesystem walks using platform-optimized syscalls.
  - Collect metadata: perms, uid/gid, ns-mtime, symlinks, hardlinks, sparse, devices/FIFOs, xattrs, ACLs.
  - Apply FilterAgent decisions during traversal to avoid unnecessary flist bloat.
- **Design Pattern**: **Visitor** for traversing filesystem entries.

---

### 7. **MetadataAgent**
- **Role**: Preserves and restores file metadata.
- **Responsibilities**:
  - Implement perms, ownership, timestamps, symlinks, hardlinks, devices, FIFOs, xattrs, and POSIX ACLs.
  - Guarantee identical behavior to rsync for default modes.
  - Provide opt-in strict vs relaxed metadata validation.
  - Cross-platform support via cfg-gated implementations (Linux, macOS).
- **Design Pattern**: **Adapter** for OS-specific metadata calls.

---

### 8. **TransportAgent**
- **Role**: Handles network communication and transport protocols.
- **Responsibilities**:
  - Implement SSH stdio transport (rsync over remote shell).
  - Implement rsync:// TCP daemon (port 873) with modules, secrets file, chroot, and uid/gid dropping.
  - Implement privilege separation and path normalization to prevent traversal attacks.
  - Manage sockets, retries, timeouts, and error recovery.
- **Design Pattern**: **Adapter** pattern for different transports.

---

### 9. **EngineAgent**
- **Role**: Core sync engine orchestrating delta generation and file application.
- **Responsibilities**:
  - Drive sender and receiver state machines.
  - Apply deltas to create or update local files with correct metadata.
  - Handle resume/partials, temp files, inplace updates, deletion policies.
  - Ensure correctness under chaotic conditions (network interruptions, vanished files).
- **Design Pattern**: **Coordinator** with **State Machine** for sender/receiver roles.

---

### 10. **DaemonAgent**
- **Role**: Provides full rsync:// server implementation.
- **Responsibilities**:
  - Parse rsyncd.conf-equivalent configs for modules, paths, comments, permissions.
  - Enforce chroot, uid/gid dropping, max connections, hosts allow/deny.
  - Provide motd, logs, lock/state, and PID file management.
  - Maintain parity with upstream rsync daemon for security and semantics.
- **Design Pattern**: **Builder** for module configuration.

---

### 11. **CLIControllerAgent**
- **Role**: Exposes the command-line interface.
- **Responsibilities**:
  - Use `clap v4` to implement all rsync flags and aliases.
  - Provide identical help text, usage, and defaults as upstream rsync.
  - Forward parsed arguments into a validated configuration (Builder pattern).
  - Enhancement flags will be introduced only after achieving parity.
- **Design Pattern**: **Builder** for options validation; **Facade** to simplify user entrypoint.

---

### 12. **LoggingAgent**
- **Role**: Provides unified logging and observability.
- **Responsibilities**:
  - Structured logs via `tracing`.
  - Support human-readable and JSON output modes.
  - Mirror rsync’s logging verbosity levels (`-v`, `--info`, `--debug`).
  - Integrate with daemon log files and system logging facilities.
- **Design Pattern**: **Observer** pattern for log sinks.

---

### 13. **TestAgent**
- **Role**: Automated testing and validation.
- **Responsibilities**:
  - Run interop tests against stock rsync (SSH + daemon).
  - Execute golden-tree comparisons across data + metadata.
  - Provide fuzzing targets for protocol and filters.
  - Perform property-based tests for filter precedence and flist integrity.
  - Run chaos and soak tests to ensure robustness under load.
- **Design Pattern**: **Harness** with plug-in tests; fuzzing + property tests integrated.

---

## Agent Collaboration

1. **CLIControllerAgent** parses arguments → builds config.
2. Config passed to **ProtocolAgent** + **TransportAgent** for setup.
3. **WalkAgent** discovers files → **FilterAgent** prunes entries → **FileListAgent** encodes list → sent via **ProtocolAgent**.
4. **EngineAgent** applies deltas using **ChecksumAgent** + **CompressionAgent**.
5. **MetadataAgent** restores file attributes.
6. **DaemonAgent** serves modules in rsync:// mode.
7. **LoggingAgent** records every stage; **TestAgent** validates correctness.

---

## Deliverables Per Agent
- Unit tests per agent.
- Integration tests across agents.
- Interop matrix tests with upstream rsync.
- Documentation in `docs/` (manpages, differences, feature_matrix).
- Fuzz + chaos test harnesses.

---

## Summary
This agent architecture provides a **clean separation of responsibilities** and explicit parity with upstream rsync (validated via interop tests). Enhancements will only be considered once parity is fully realized. Each agent can be individually tested, audited, and fuzzed, ensuring both **robustness** and **maintainability**.

---

## Development Guidelines

### Coding Conventions
- Format code with `cargo fmt --all`.
- Lint with `cargo clippy --all-targets --all-features -- -D warnings`.

### Mandatory Checks
- Install `cargo-nextest` with `cargo install cargo-nextest --locked` if it is not already available.
- Run `cargo nextest run --workspace --no-fail-fast` (and `--all-features`) and ensure all tests pass.
- Execute `make verify-comments` to validate file header comments.
- Use `make lint` to confirm formatting.

### References
- See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.
- CI workflow: [.github/workflows/ci.yml](.github/workflows/ci.yml)
