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

### Trailing slash semantics

Just like `rsync`, adding a trailing slash to the source path changes what is
copied:

- `rsync-rs src/ dest/` copies the *contents* of `src` into `dest`.
- `rsync-rs src dest/` creates a `dest/src` directory containing the original
  files.
- `rsync-rs remote:/src/ local/` pulls the contents of `/src` from the remote
  host into `local`.

## Flags

- `-r, --recursive` *(default: off)* – copy directories recursively.
- `-n, --dry-run` *(default: off)* – perform a trial run with no changes made.
- `-v, --verbose` *(default: off)* – increase logging verbosity.
- `-q, --quiet` *(default: off)* – suppress non-error messages.
- `--delete` *(default: off)* – remove extraneous files from the destination.
- `-c, --checksum` *(default: off)* – use full checksums to determine file changes.
- `-z, --compress` *(default: off)* – compress file data during the transfer.
- `--stats` *(default: off)* – display transfer statistics on completion.
- `--no-motd` *(default: off)* – suppress the message of the day when connecting to a daemon.
- `--config <FILE>` *(default: `~/.config/rsync-rs/config.toml`)* – supply a custom configuration file.
- `-e, --rsh <COMMAND>` *(default: `ssh`)* – specify the remote shell to use.

### Daemon and server modes

- `--daemon` – run as an rsync daemon accepting incoming connections *(default: off).* 
- `--server` – enable server behavior; typically invoked internally when a remote peer connects *(default: off).* 

### Remote shell

`-e, --rsh` selects the program used to connect to remote hosts. The command
line takes precedence, followed by the `RSYNC_RSH` environment variable; if
neither is provided, `ssh` is used.

### Deletion flags

Deletion options control how extraneous files at the destination are removed.
All deletion flags are disabled by default.

- `--delete` – remove files during the transfer.
- `--del` – an alias for `--delete-during`.
- `--delete-before`, `--delete-during`, `--delete-after`, and `--delete-delay` –
  adjust when deletions occur.
- `--delete-excluded` – also delete files that are excluded by filters.

### Advanced transfer options

These flags mirror `rsync(1)` features that fine‑tune how data is copied.

- `--partial` – keep partially transferred files instead of discarding them.
- `--partial-dir <DIR>` – place partial files in `DIR`.
- `--append` / `--append-verify` – append data to existing files rather than
  replacing them.
- `--sparse` – convert long sequences of zero bytes into holes and preserve
  holes in sparse source files. The destination filesystem must support sparse
  files. See `tests/cli.rs::sparse_files_created` and
  `tests/cli.rs::sparse_files_preserved` for examples.
- `--bwlimit <RATE>` – throttle I/O bandwidth to `RATE` bytes per second using a
  token bucket rate limiter.
- `--link-dest <DIR>` / `--copy-dest <DIR>` – hard‑link or copy unchanged
  files from `DIR`.

Flags such as `-a`, `-R`, `-P`, and `--numeric-ids` mirror their `rsync`
behavior. See `docs/differences.md` for a summary of supported options.

For a comprehensive list of available flags and their current support status,
see the [CLI flag reference](cli/flags.md).

## Configuration precedence

1. Command-line flags
2. Environment variables (prefixed with `RSYNC_RS_`)
3. Configuration file (defaults to `~/.config/rsync-rs/config.toml`)
4. Built-in defaults

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
