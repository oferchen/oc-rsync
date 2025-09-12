# Core Crate Overview

The `oc-rsync-core` crate exposes shared building blocks used across the
project.  It provides facade modules re-exporting existing implementations:

- `fs`: file and metadata helpers from `meta`.
- `filter`: path and name filtering utilities from `filters`.
- `transfer`: transfer engine primitives from `engine`.
- `hardlink`: hard link tracking support from `meta`.
- `message`: protocol message types from `protocol`.
- `config`: synchronization options from `engine`.
- `metadata`: high-level metadata options from `meta`.

These modules offer a stable surface for higher level crates such as the
command line interface and transport layers.
