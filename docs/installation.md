# Installation

## Required system packages

To build oc-rsync from source you need a working C toolchain, compression libraries, and ACL headers:

- `build-essential` (provides `gcc` and `ld`)
- `libzstd-dev`
- `zlib1g-dev`
- `libacl1-dev`

Install them on Debian/Ubuntu with:

```bash
sudo apt-get update
sudo apt-get install -y build-essential libzstd-dev zlib1g-dev libacl1-dev
```

Run `scripts/preflight.sh` to verify these dependencies before compiling.

If your system lacks the `libacl` development package, build without ACL
support using:

```bash
cargo build --no-default-features --features xattr
```

