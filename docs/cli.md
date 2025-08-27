# Command Line Interface

The `rsync-rs` binary aims to mirror the familiar `rsync` experience. An
overview of project goals and features is available in the
[README](../README.md#in-scope-features).

## Usage

```sh
rsync-rs [OPTIONS] <SRC> <DEST>
```

### Examples

- Local directory sync:
  ```sh
  rsync-rs ./src ./backup
  ```
- Remote sync over SSH:
  ```sh
  rsync-rs ./src user@example.com:/var/www
  ```
- Dry run with statistics:
  ```sh
  rsync-rs -n --stats ./src remote:/dst
  ```

## Flags

- `-r, --recursive` – copy directories recursively.
- `-n, --dry-run` – perform a trial run with no changes made.
- `-v, --verbose` – increase logging verbosity.
- `--delete` – remove extraneous files from the destination.
- `--checksum` – use full checksums to determine file changes.
- `--config <FILE>` – supply a custom configuration file.

## Configuration precedence

1. Command-line flags
2. Environment variables (prefixed with `RSYNC_RS_`)
3. Configuration file (defaults to `~/.config/rsync-rs/config.toml`)

Settings specified earlier in the list override later sources.
