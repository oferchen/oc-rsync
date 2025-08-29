# AGENTS.MD — rsync_rs

**Purpose:** This document instructs an autonomous coding agent (e.g., “Codex”) how to drive `rsync_rs` to production with **default rsync 3.2.x compatibility** (protocol v31) and **opt-in enhancements** that do **not** affect interop with stock rsync.

---

## Mission

Deliver a **pure-Rust**, **no-FFI** implementation that:
1) Mirrors **rsync 3.2.x** CLI, behavior, exit codes, and daemon semantics by default.  
2) Fully interoperates with stock rsync over **SSH** and **rsync:// daemon**.  
3) Offers an **opt-in** `--modern` mode (only when both peers are `rsync_rs`) for improved hashing/compression and optional advanced features.

**Non-goals (for now):** obscure flags (`--fuzzy`, batch mode, `--copy-devices`), Windows ACL fidelity. These may be scheduled after parity.

---

## Ground Rules

- **Compatibility first.** Default wire protocol and CLI must match rsync 3.2.x (v31).
- **No FFI.** Rust only; no deprecated crates.
- **Enhancements are gated** behind `--modern` and **must not** change default interop.
- **Security by default.** Path normalization, safe daemon jail, privilege drop, timeouts.
- **Tests with every feature.** Fuzz/proptest for parsers and protocol.
- **Small, cohesive crates.** Clean code, single responsibility, documented public APIs.

---

## Top-Level Deliverables

- `bin/rsync-rs/` (single binary: client + `--daemon`)
- `docs/`  
  - `feature_matrix.md` (rsync flags vs support & tests)  
  - `cli.md` (argument parity; examples)  
  - `daemon.md` (rsyncd semantics)  
  - `differences.md` (only `--modern`)  
  - `SECURITY.md` (privilege model, jail, auth, limits)
- `tests/interop/` (scripted grid vs stock rsync)
- `fuzz/` (protocol, filelist, filters)
- `man/` (`rsync-rs(1)`, `rsync-rs.conf(5)`)
- CI artifacts (Linux x86_64/arm64, macOS)

---

## Architecture (expected crates)

- `crates/cli` — clap v4; 1:1 flags/aliases/defaults with rsync 3.2.x.  
- `crates/protocol` — v31 frames, phases, mux/demux, keep-alive, progress, exit codes.  
- `crates/checksums` — rolling checksum + strong digests (MD5/SHA1 for compat; **BLAKE3** only under `--modern`). Strategy pattern.  
- `crates/compress` — zlib (default), zstd (if negotiated).  
- `crates/filters` — include/exclude grammar + per-dir `.rsync-filter` merges (order-dependent).  
- `crates/walk` — streaming file-list, delta-encoded paths/attrs, bounded memory.  
- `crates/meta` — perms, uid/gid, symlinks, **hardlinks**, **sparse** (SEEK_HOLE/DATA with fallbacks), devices/FIFOs, **xattrs**, **POSIX ACLs**, ns-mtimes.  
- `crates/engine` — sender/receiver state machines, block-size heuristics, resume/partials, `--temp-dir` vs `--inplace`.  
- `crates/transport` — SSH stdio; rsync:// TCP daemon (port 873) with modules, secrets file, chroot jail, uid/gid drop, hosts allow/deny.  
- `crates/logging` — `tracing` (+ json mode).

---

## Execution Plan

### Phase 0 — Repository Audit (autorun on checkout)
- Enumerate current modules, tests, fuzzers, and CI.
- Produce `docs/gaps.md`: implemented vs missing features; dead/unreachable code; TODOs; test coverage; CI holes.
- Produce initial `docs/feature_matrix.md` from **rsync(1)** and **rsyncd.conf(5)** (3.2.x). Mark unknowns as ❓.

### Phase 1 — CLI Parity
- Implement complete clap schema mirroring rsync flags/aliases/defaults.  
- Must include (non-exhaustive):  
  `-a`, `-rltgoD`, `-v`, `-n`, `-i`, `--stats`, `--delete*`, `--ignore-existing`, `--existing`,  
  `--partial`, `--partial-dir`, `--inplace`, `--temp-dir`,  
  `--links`, `--hard-links`, `--copy-links`,  
  `--devices`, `--specials`,  
  `--numeric-ids`, `--times`, `--perms`, `--owner`, `--group`, `--chmod`,  
  `-z/--compress`, `--compress-choice=zlib|zstd`, `--checksum`, `--whole-file`, `--block-size=`,  
  `-e/--rsh`, `--daemon`, `--server`, `--ipv4/--ipv6`, rsync:// URL form.  
- Add `--modern` (enhanced mode). **Default off.**

**Acceptance:** `rsync-rs --help` diffs cleanly against `rsync --help` (names, shorts, defaults).

### Phase 2 — Protocol v31 Core
- Frame codec, phases, multiplexing, keep-alives, progress, exit-code mapping.
- Generate **golden transcripts** by recording real rsync sessions; ensure decode/encode parity.

**Acceptance:** Local loopback sync for small trees; golden transcript tests pass.

### Phase 3 — Engine, Walk, Metadata (local)
- Implement sender/receiver state machines; rsync block heuristics; bounded memory.  
- Resume/partials; `--temp-dir` vs `--inplace`.  
- Metadata: symlinks, **hardlinks**, **sparse**, devices/FIFOs, uid/gid mapping (`--numeric-ids`), ns-mtimes.  
- Introduce **xattrs** + **POSIX ACLs** (Linux first; cfg-gated per OS).

**Acceptance:** Golden tree comparisons (content + stat/xattr/ACL/hardlink/sparse equivalence).

### Phase 4 — Filters (full)
- Parse/match include/exclude; per-dir `.rsync-filter`; order-sensitive semantics.  
- Add fuzzers (`cargo-fuzz`) + property tests (`proptest`).

**Acceptance:** Cross-checked against stock rsync behavior on synthetic suites.

### Phase 5 — Transports
- SSH stdio transport (exec remote server).  
- rsync:// daemon: modules, auth (`secrets file`), chroot jail, uid/gid drop, port 873, hosts allow/deny, logs/locks/pids, motd.

**Acceptance:** Interop matrix (ssh + daemon) vs stock rsync across common flag sets.

### Phase 6 — Compression & Negotiation
- zlib default; zstd if both ends advertise.  
- **Do not** break stock interop; fall back gracefully.

**Acceptance:** Mixed-peer tests (stock rsync ↔ rsync_rs) pass with/without compression.

### Phase 7 — Robustness & Chaos
- Fault injection: short reads/writes, network flaps, vanished files.  
- Large-tree streaming; backpressure; memory caps.  
- Exit-code semantics and diagnostics consistent with rsync.

**Acceptance:** Soak tests; chaos suite green; resource budgets documented.

### Phase 8 — Modern Mode (opt-in)
- Capability negotiation enabling:  
  - strong hash **BLAKE3** (replaces SHA1/MD5 only when both ends support),  
  - zstd-preferred,  
  - (optional) CDC/manifest reuse **without** altering default protocol when talking to stock rsync.  
- Document in `docs/differences.md`.

**Acceptance:** Modern↔modern tests pass; modern↔stock auto-fallback to compat.

### Phase 9 — Docs, Manpages, CI, Release
- `docs/cli.md`, `docs/daemon.md`, `docs/differences.md`, `docs/feature_matrix.md`, `SECURITY.md`.  
- Generate `man/rsync-rs.1` and `man/rsync-rs.conf.5`.  
- CI: Linux x86_64/arm64 + macOS; fuzz nightly; release artifacts.

---

## Test Plan

- **Interop Grid (`tests/interop/`)**  
  - rsync 3.2.x client↔server ⇄ `rsync_rs` (SSH + daemon).  
  - Flag subsets covering permissions, times, deletes, filters, compression, links, xattrs/ACLs, sparse, numeric-ids.  
  - Verify itemized changes, stats output, and exit codes.

- **Golden Trees**  
  - Byte content + `stat` parity, xattrs/ACL equality, link/sparse correctness.  
  - Deep directories, millions of entries, churny trees.

- **Chaos & Recovery**  
  - Inject connection resets, timeouts, partials/resume.  
  - Ensure `--partial`, `--partial-dir`, `--inplace` correctness.

- **Fuzzing**  
  - Protocol frame decoder, filelist decoder, filter grammar.

---

## Security Requirements

- **Daemon**: chroot jail per module, privilege drop (uid/gid), strict path normalization, safe `secrets file` handling, hosts allow/deny.  
- **Transport**: “shell-clean” behavior (no banner noise), timeouts, size limits, defensive decoding.  
- **Defaults**: conservative; explicit opt-in for risky flags (`--inplace`, `--devices`, `--specials`).

---

## Definition of Done (Parity)

- `feature_matrix.md`: All rsync 3.2.x options either ✔ (parity) or 💤 (explicitly deferred).  
- Interop tests: green for SSH & daemon against stock rsync.  
- Golden comparisons: content + metadata parity (as per matrix).  
- Fuzzers: stable for N runner-hours/day with no new crashes.  
- Docs & manpages: complete; CLI help matches rsync.  
- `--modern` documented and safely negotiated; default interop unaffected.

---

## Issue Labels & Workflow

- `epic:cli`, `epic:protocol`, `epic:engine`, `epic:filters`, `epic:metadata`, `epic:transport`, `epic:daemon`, `epic:modern`, `epic:interop`, `epic:fuzz`  
- `type:bug`, `type:feature`, `type:doc`, `type:test`, `type:security`  
- `prio:P0` (blocking parity), `prio:P1` (important), `prio:P2` (post-parity)

**PR gates:** tests + docs for any new flag/behavior; interop delta explained; `differences.md` unchanged unless `--modern`.

---

## Coding Standards (Rust)

- Stable Rust, 2021+ edition.  
- `clippy` + `rustfmt` clean; deny warnings on CI.  
- `tracing` for logs; avoid `println!`.  
- Avoid async until daemon is stable; then `tokio` if needed.  
- No global mutable state; pass context explicitly.  
- Panics only for unrecoverable programmer errors; IO/protocol errors are `Result`.

---

## Quickstart Commands (for the Agent)

1. **Audit & matrices**
   - Generate `docs/gaps.md`, `docs/feature_matrix.md`, `docs/cli.md`.
2. **Run interop locally (once implemented)**
   - `./tools/interop/run_grid.sh --rsync /usr/bin/rsync --rsync-rs ./target/release/rsync-rs`
3. **Fuzz**
   - `cargo fuzz run protocol_decoder`  
   - `cargo fuzz run filter_parser`
4. **Package**
   - `cargo build --release`  
   - Produce manpages/completions under `man/`.

---

## Notes

- Where rsync behavior is undocumented but observable, capture **golden transcripts** and codify with tests.  
- Never change defaults to suit enhancements. All enhancements must be **explicit and negotiated**.

