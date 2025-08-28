# Command Line Interface

The `rsync-rs` binary aims to mirror the familiar `rsync` experience. An
overview of project goals and features is available in the
[README](../README.md#in-scope-features), and a high-level summary of CLI goals
lives in the [README's CLI section](../README.md#cli).

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
- Sync using an explicit config file:
  ```sh
  rsync-rs --config ./rsync-rs.toml ./src remote:/dst
  ```

## Flags

- `-r, --recursive` – copy directories recursively.
- `-n, --dry-run` – perform a trial run with no changes made.
- `-v, --verbose` – increase logging verbosity.
- `--delete` – remove extraneous files from the destination.
- `-c, --checksum` – use full checksums to determine file changes.
- `--stats` – display transfer statistics on completion.
- `--config <FILE>` – supply a custom configuration file.

For a comprehensive list of available flags and their current support status,
see the [CLI flag reference](cli/flags.md).

## Configuration precedence

1. Command-line flags
2. Environment variables (prefixed with `RSYNC_RS_`)
3. Configuration file (defaults to `~/.config/rsync-rs/config.toml`)

Settings specified earlier in the list override later sources.

## Filters

`rsync-rs` supports include and exclude rules using the same syntax as
`rsync`. Rules can be supplied on the command line with `--filter` or placed in
`.rsync-filter` files located throughout the source tree. When walking
directories the tool automatically loads any `.rsync-filter` files and merges
their rules with those from parent directories. Rules in deeper directories take
precedence over rules defined higher up.

### Example

```
project/
├── .rsync-filter      # contains: - *.tmp
└── logs/
    ├── .rsync-filter  # contains: + keep.tmp
    ├── keep.tmp
    └── other.tmp
```

In this layout, `keep.tmp` is included because the rule in `logs/.rsync-filter`
overrides the root exclusion. The file `other.tmp` remains excluded.
