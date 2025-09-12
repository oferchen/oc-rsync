
# oc-rsync — Agents

> **Scope:** This document describes the small, focused **agents** (scripts and jobs) that enforce parity, quality, and interoperability for **oc-rsync** (Rust 2024). Each agent has a single responsibility, deterministic I/O, and a crisp success/failure contract.

- **Repository:** https://github.com/oferchen/oc-rsync  
- **Parity Target:** Upstream rsync **3.4.1** (protocol **32**) — **byte-for-byte** parity of user-visible behavior.  
- **Edition:** Rust **2024** (workspace-wide).  
- **Order of Precedence:** This doc follows the project’s consolidated brief; **no agent may change observable behavior**.

---

## 1) Agent Model

Each agent is a **self-contained tool** (usually a shell script) invoked locally or by CI:
- **Single purpose** (“do one thing well”).  
- **Deterministic**: honors `LC_ALL=C`, fixed `COLUMNS=80`, and pinned inputs.  
- **No placeholders**: complete, runnable commands; non-zero exit codes on failure.  
- **Stateless**: no persistent state besides explicit artifacts/logs under `/tmp` or `target/`.

**Common environment (CI and local):**
```sh
export LC_ALL=C
export COLUMNS=80
export RUSTFLAGS=-Dwarnings
export CARGO_TERM_COLOR=always
```

**Prerequisites (install *before* building or running agents):**
- Ubuntu/Debian: `build-essential pkg-config curl ca-certificates openssh-client libacl1-dev libzstd-dev zlib1g-dev libxxhash-dev`
- AlmaLinux/RHEL/Fedora: `gcc make pkgconfig curl ca-certificates openssh-clients libacl-devel libzstd-devel zlib-devel xxhash-devel`
- Arch Linux: `base-devel pkgconf curl ca-certificates openssh acl zstd zlib xxhash`

These packages enable building upstream rsync for interop and provide system libs when linking compressors/ACLs.

---

## 2) Lint & Hygiene Agents

### 2.1 `comment_lint` Agent
- **Script:** `tools/comment_lint.sh`
- **Purpose:** Enforce the **single-header-line** rule (`// crates/...` or `// src/...`) and ban any further `//` comments in Rust files.
- **Run locally:**
  ```sh
  bash tools/comment_lint.sh
  ```
- **Pass criteria:** No diagnostic output; exit `0`.
- **Fail criteria:** Prints offending files; exit `1`.

### 2.2 `enforce_limits` Agent
- **Script:** `tools/enforce_limits.sh`
- **Purpose:** Enforce **LoC caps** (target ≤400; hard **≤600** lines) and comment policy.
- **Config:** `MAX_RUST_LINES` (default `600`).
- **Run locally:**
  ```sh
  MAX_RUST_LINES=600 bash tools/enforce_limits.sh
  ```

### 2.3 `check_layers` Agent
- **Script:** `tools/check_layers.sh`
- **Purpose:** Enforce module **layering**: `checksums, compress, filters, meta, protocol → transport, engine, logging → cli`. No upward deps.
- **Run locally:**
  ```sh
  bash tools/check_layers.sh
  ```

### 2.4 `no_placeholders` Agent
- **Script:** `tools/no_placeholders.sh`
- **Purpose:** Ban `todo!`, `unimplemented!`, `FIXME`, `XXX`, and obvious placeholder panics in Rust sources.
- **Run locally:**
  ```sh
  bash tools/no_placeholders.sh
  ```

---

## 3) Build & Test Agents

### 3.1 `lint` Agent (fmt + clippy)
- **Invoker:** CI job `lint` (see workflow).  
- **Purpose:** Enforce formatting and deny warnings.
- **Run locally:**
  ```sh
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -Dwarnings
  ```

### 3.2 `test-linux` Agent (coverage-gated)
- **Purpose:** Run unit/integration tests and enforce **≥95%** line/block coverage.
- **Run locally (example):**
  ```sh
  rustup component add llvm-tools-preview
  cargo install cargo-llvm-cov
  cargo llvm-cov --workspace --lcov --output-path coverage.lcov --fail-under-lines 95
  ```
- **Artifacts:** `coverage.lcov`

### 3.3 `build-matrix` Agent
- **Purpose:** Release builds for Linux/macOS/Windows (x86_64 + aarch64 as applicable).  
- **Run locally (Linux example):**
  ```sh
  cargo build --release --workspace
  ```

### 3.4 `package-linux` Agent (+ SBOM)
- **Purpose:** Build `.deb`, `.rpm`, and generate CycloneDX SBOM.
- **Run locally (examples):**
  ```sh
  cargo install cargo-deb cargo-rpm
  cargo deb --no-build
  cargo rpm build
  cargo install cyclonedx-bom || true
  cyclonedx-bom -o target/sbom/oc-rsync.cdx.json
  ```

---

## 4) Interoperability Agents (Loopback **rsync://127.0.0.1** Only)

Interoperability agents validate **oc↔upstream** compatibility for **protocols 30/31/32** using upstream **3.0.9**, **3.1.3**, and **3.4.1**. **Network binds to 127.0.0.1 only.**

> **Scripts live under:** `scripts/interop/`

### 4.1 `build_upstream` Agent
- **Script:** `scripts/interop/build_upstream.sh`
- **Purpose:** Build upstream rsync versions locally under a controlled prefix.
- **Usage:**
  ```sh
  scripts/interop/build_upstream.sh /tmp/rs-build /tmp/rs-install 3.0.9 3.1.3 3.4.1
  ```
- **Outputs:** `/tmp/rs-install/<ver>/bin/rsync` for each version.

### 4.2 `start_daemons` Agent
- **Script:** `scripts/interop/start_daemons.sh`
- **Purpose:** Start upstream rsync daemons bound to `127.0.0.1` on fixed ports (8730, 8731, 8732).
- **Usage:**
  ```sh
  scripts/interop/start_daemons.sh /tmp/rs-install 3.0.9 3.1.3 3.4.1
  ```
- **State:** Data under `/tmp/rsdata/<ver>/`; PIDs under `/tmp/rsdaemons/`.

### 4.3 `run` Agent
- **Script:** `scripts/interop/run.sh`
- **Purpose:** Run the **oc client→upstream daemon** and **upstream client→oc daemon** matrix across proto 30/31/32 and compare directory trees.
- **Usage:** (after building oc-rsync)
  ```sh
  cargo build --release -p oc-rsync
  scripts/interop/run.sh /tmp/rs-install 3.0.9 3.1.3 3.4.1 target/release/oc-rsync
  ```
- **Behavior:** Generates random trees; performs transfers in both directions; diffs trees.

### 4.4 `validate` Agent
- **Script:** `scripts/interop/validate.sh`
- **Purpose:** Sanity-check daemon logs exist and non-empty.
- **Usage:**
  ```sh
  scripts/interop/validate.sh
  ```

**Expected Outcome:** All transfers succeed; directory trees and metadata match; stdout/stderr and exit codes match upstream behavior; logs present.

---

## 5) Repository Hygiene Agents

### 5.1 `no-binaries` Agent
- **Purpose:** Fail CI if any tracked file appears binary (heuristic or `git diff --check`).  
- **Local hint:** Use `git diff --numstat` to spot non-text additions and prevent accidental binary commits.

### 5.2 `readme-version` Agent
- **Purpose:** Verify `README.md` states “Compatible with rsync **3.4.1** (protocol **32**).”  
- **Implementation:** Simple grep in CI.

---

## 6) Workflow Wiring (GitHub Actions Overview)

> **Note:** Exact workflow YAML may evolve; the following reflects the **jobs** that invoke agents.

- **lint**: runs `fmt`, `clippy`, then `tools/comment_lint.sh`, `tools/enforce_limits.sh`, `tools/check_layers.sh`, `tools/no_placeholders.sh`.
- **test-linux**: runs `cargo llvm-cov` with `--fail-under-lines 95` and uploads `coverage.lcov`.
- **build-matrix**: cross-OS builds to verify portability (no behavior change).
- **package-linux**: produces `.deb`/`.rpm` and SBOM.
- **interop-linux**: invokes `scripts/interop/*.sh` to verify oc↔upstream over loopback `rsync://` across protocols 30/31/32.
- **no-binaries** / **readme-version**: simple grep/filters.

All jobs share:
```yaml
env:
  LC_ALL: C
  COLUMNS: 80
  RUSTFLAGS: -Dwarnings
  CARGO_TERM_COLOR: always
```

And CI uses a concurrency group to cancel superseded runs:
```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

---

## 7) Agent Contracts (Inputs/Outputs/Exit Codes)

| Agent | Inputs | Outputs | Success (exit 0) | Failure (exit ≠0) |
|---|---|---|---|---|
| comment_lint | repo files | diagnostics (stdout) | no diagnostics | lists offending files |
| enforce_limits | `MAX_RUST_LINES` | diagnostics | all ≤ limit | report offending files |
| check_layers | cargo metadata | diagnostics | no upward deps | reports layering errors |
| no_placeholders | repo files | diagnostics | none found | reports placeholder hits |
| lint (fmt+clippy) | Rust sources | diagnostics | clean | warnings/errors |
| test-linux | tests | `coverage.lcov` | coverage ≥95% | test/coverage failure |
| build-matrix | sources | release artifacts | builds succeed | build failure |
| package-linux | workspace | `.deb/.rpm` + SBOM | artifacts present | packaging failure |
| build_upstream | versions | installed rsync | binaries present | build failure |
| start_daemons | installed rsync | running daemons | ports alive | daemon failed |
| run | oc binary + daemons | diff result | trees identical | diff or transfer fail |
| validate | logs | diagnostics | logs present | logs missing/empty |

---

## 8) Running Everything Locally (Happy Path)

```sh
# 0) Prereqs
#   (Use the package list for your OS; see §1 above)

# 1) Lint & hygiene
bash tools/comment_lint.sh
bash tools/enforce_limits.sh
bash tools/check_layers.sh
bash tools/no_placeholders.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -Dwarnings

# 2) Tests & coverage (≥95%)
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --lcov --output-path coverage.lcov --fail-under-lines 95

# 3) Build oc-rsync
cargo build --release -p oc-rsync

# 4) Interop (loopback rsync:// only)
scripts/interop/build_upstream.sh /tmp/rs-build /tmp/rs-install 3.0.9 3.1.3 3.4.1
scripts/interop/start_daemons.sh /tmp/rs-install 3.0.9 3.1.3 3.4.1
scripts/interop/run.sh /tmp/rs-install 3.0.9 3.1.3 3.4.1 target/release/oc-rsync
scripts/interop/validate.sh
```

If any step fails, read the agent’s diagnostics (stdout/stderr). Agents are intentionally terse and specific.

---

## 9) Security & Safety Notes

- **Behavioral Parity First:** No agent may alter CLI/help text, protocol, messages, or exit codes. Agents verify; they do not mutate user-visible behavior.
- **Loopback Only for Interop:** All rsync daemons bind to `127.0.0.1` and are not exposed externally.
- **Privileges:** Packaging installs a hardened systemd unit; development agents do not require elevated privileges beyond package installation.
- **Determinism:** Always run with `LC_ALL=C` and `COLUMNS=80` to keep output snapshots stable.

---

## 10) Ownership & Maintenance

- **Where:** Agents live under `tools/`, `scripts/interop/`, and CI workflow files under `.github/workflows/`.
- **Policy:** Any change to agents must preserve their single responsibility and deterministic outputs. Keep diffs small; document intent in PR descriptions.

---

**End of `agents.md`**
