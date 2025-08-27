# Feature Overview

This document tracks the major capabilities planned for the project.

## Implemented
- Repository scaffolding and documentation.
- Directory traversal and checksum crates that generate file signatures.
- CLI support for syncing local directories using those signatures, covered by tests.

## Planned
- Core rolling-checksum and delta-transfer engine.
- Secure network transport compatible with existing rsync.
- Flexible filtering, compression, and progress reporting.
- Cross-platform file system abstractions.
