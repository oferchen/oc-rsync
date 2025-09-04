# Installation

## Required system packages

To build oc-rsync from source you need a working C toolchain and compression libraries:

- `build-essential` (provides `gcc` and `ld`)
- `libzstd-dev`
- `zlib1g-dev`

Install them on Debian/Ubuntu with:

```bash
sudo apt-get update
sudo apt-get install -y build-essential libzstd-dev zlib1g-dev
```

Run `scripts/preflight.sh` to verify these dependencies before compiling.

