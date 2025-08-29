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
- Mirror with exclusions and deletions:
  ```sh
  rsync-rs -a --delete --exclude '.cache/' ./src/ ./mirror/
  ```
- Incremental backup using hard links:
  ```sh
  rsync-rs -a --link-dest=../prev --compare-dest=../base ./src/ ./snapshot/
  ```
- Throttled modern compression:
  ```sh
  rsync-rs -az --modern --bwlimit=1m ./src/ host:/archive/
  ```
- Tune delta block size:
  ```sh
  rsync-rs -B 65536 ./src remote:/dst
  ```

### Trailing slash semantics

Just like `rsync`, adding a trailing slash to the source path changes what is
copied:

- `rsync-rs src/ dest/` copies the *contents* of `src` into `dest`.
- `rsync-rs src dest/` creates a `dest/src` directory containing the original
  files.
- `rsync-rs remote:/src/ local/` pulls the contents of `/src` from the remote
  host into `local`.

## Options

| Short | Long | Default | Description |
|-------|------|---------|-------------|
| `-v` | `--verbose` | off | increase verbosity |
| `-q` | `--quiet` | off | suppress non-error messages |
| | `--no-motd` | off | suppress daemon-mode MOTD |
| `-c` | `--checksum` | off | skip based on checksum, not mod-time & size |
| `-a` | `--archive` | off | archive mode is -rlptgoD (no -A,-X,-U,-N,-H) |
| `-r` | `--recursive` | off | recurse into directories |
| `-R` | `--relative` | off | use relative path names |
| `-I` | `--inplace` | off | update destination files in-place |
| | `--append` | off | append data onto shorter files |
| | `--append-verify` | off | --append w/old data in file checksum |
| `-l` | `--links` | off | copy symlinks as symlinks |
| | `--hard-links` | off | preserve hard links |
| | `--perms` | off | preserve permissions |
| | `--chmod=CHMOD` | none | affect file and/or directory permissions |
| | `--acls` | off | preserve ACLs (implies --perms) |
| | `--xattrs` | off | preserve extended attributes |
| | `--owner` | off | preserve owner (super-user only) |
| | `--group` | off | preserve group |
| | `--devices` | off | preserve device files (super-user only) |
| | `--specials` | off | preserve special files |
| | `--times` | off | preserve modification times |
| `-U` | `--atimes` | off | preserve access (use) times |
| `-N` | `--crtimes` | off | preserve create times (newness) |
| `-S` | `--sparse` | off | turn sequences of nulls into sparse blocks |
| `-n` | `--dry-run` | off | perform a trial run with no changes made |
| `-e` | `--rsh=COMMAND` | `ssh` | specify the remote shell to use |
| | `--rsync-path=PROGRAM` | none | specify the rsync to run on remote machine |
| | `--delete` | off | delete extraneous files from dest dirs |
| | `--delete-before` | off | receiver deletes before transfer, not during |
| | `--delete-during` (`--del`) | off | receiver deletes during the transfer |
| | `--delete-delay` | off | find deletions during, delete after |
| | `--delete-after` | off | receiver deletes after transfer, not during |
| | `--delete-excluded` | off | also delete excluded files from dest dirs |
| `-b` | `--backup` | off | make backups of replaced or removed files |
| | `--backup-dir=DIR` | none | store backups under DIR |
| | `--partial` | off | keep partially transferred files |
| | `--partial-dir=DIR` | none | put a partially transferred file into DIR |
| | `--numeric-ids` | off | don't map uid/gid values by user/group name |
| | `--compare-dest=DIR` | none | also compare destination files relative to DIR |
| | `--copy-dest=DIR` | none | ... and include copies of unchanged files |
| | `--link-dest=DIR` | none | hardlink to files in DIR when unchanged |
| | `--config=FILE` | none | specify alternate config file |
| `-z` | `--compress` | off | compress file data during the transfer |
| | `--compress-choice=STR` | auto | choose the compression algorithm |
| | `--compress-level=NUM` | auto | explicitly set compression level |
| | `--modern` | off | enable zstd compression and BLAKE3 checksums (rsync-rs specific) |
| `-f` | `--filter=RULE` | none | add a file-filtering RULE |
| `-F` | | off | same as --filter='dir-merge /.rsync-filter' repeated: --filter='- .rsync-filter' |
| | `--exclude=PATTERN` | none | exclude files matching PATTERN |
| | `--exclude-from=FILE` | none | read exclude patterns from FILE |
| | `--include=PATTERN` | none | don't exclude files matching PATTERN |
| | `--include-from=FILE` | none | read include patterns from FILE |
| | `--files-from=FILE` | none | read list of source-file names from FILE |
| `-0` | `--from0` | off | all *-from/filter files are delimited by 0s |
| | `--stats` | off | give some file-transfer stats |
| | `--progress` | off | show progress during transfer |
| `-P` | | off | same as --partial --progress |
| | `--password-file=FILE` | none | read daemon-access password from FILE |
| | `--bwlimit=RATE` | none | limit socket I/O bandwidth |
| `-B` | `--block-size=SIZE` | 1024 | set block size used for rolling checksums |
| | `--daemon` | off | run as an rsync daemon |
| | `--module NAME=PATH` | none | module declaration (daemon mode) |
| | `--address=ADDR` | 0.0.0.0 | bind address for daemon |
| | `--port=PORT` | 873 | port to listen on (daemon mode) |
| | `--secrets-file=FILE` | none | path to secrets file for authentication |
| | `--hosts-allow=LIST` | none | list of hosts allowed to connect |
| | `--hosts-deny=LIST` | none | list of hosts denied from connecting |
| | `--log-file=FILE` | none | log what we're doing to the specified FILE |
| | `--log-file-format=FMT` | none | log updates using the specified FMT |
| | `--motd=FILE` | none | path to message of the day file |

Flags such as `-P` and `--numeric-ids` mirror their `rsync` behavior. See the [CLI flag reference](cli/flags.md) for implementation details and notes.

### Permission tweaks with `--chmod`

The `--chmod=CHMOD` option adjusts permission bits on transferred files. The
syntax mirrors `chmod` and supports comma-separated rules. Prefix a rule with
`D` or `F` to limit it to directories or files respectively.

Examples:

- `--chmod=Du+rwx` adds user read/write/execute for directories.
- `--chmod=F644` forces regular files to mode `644`.
- `--chmod=ug=rwX,o=rX` sets user and group read/write and conditional execute while
  giving others read access.

Multiple `--chmod` flags accumulate.

## Remote shell

The `--rsh` (`-e`) flag accepts an arbitrary shell command. The command string
is parsed using shell-style quoting, allowing multiple arguments and embedded
quotes much like GNU `rsync`. Leading `VAR=value` tokens set environment
variables for the spawned command. For example:

```
rsync-rs -e 'RUST_LOG=debug ssh -p 2222 -o "StrictHostKeyChecking=no"' src dst
```

During the connection handshake `rsync-rs` also forwards any environment
variables from its own process whose names begin with `RSYNC_`, mirroring
`rsync`'s environment propagation behavior.

## Configuration precedence

1. Command-line flags
2. Environment variables (prefixed with `RSYNC_RS_`)
3. Configuration file provided via `--config`
4. Built-in defaults

No configuration file is loaded unless `--config` is supplied. Settings
specified earlier in the list override later sources.

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
