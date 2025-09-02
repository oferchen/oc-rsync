# Command Line Interface

The `oc-rsync` binary aims to mirror the familiar `rsync` experience. An
overview of project goals and features is available in the
[README](../README.md#in-scope-features), and a high-level summary of CLI goals
lives in the [README's CLI section](../README.md#cli).

## Usage

```sh
oc-rsync [OPTIONS] "<SRC>" "<DEST>"
```

### Examples

- Local directory sync:

  ```sh
  oc-rsync "./src" "./backup"
  ```

- Remote sync over SSH:

  ```sh
  oc-rsync "./src" "user@example.com:/var/www"
  ```

- Dry run with statistics:

  ```sh
  oc-rsync -n --stats "./src" "remote:/dst"
  ```

- Sync using an explicit config file:

  ```sh
  oc-rsync --config "./oc-rsync.toml" "./src" "remote:/dst"
  ```

- Mirror with exclusions and deletions:

  ```sh
  oc-rsync -a --delete --exclude '.cache/' "./src/" "./mirror/"
  ```

- Incremental backup using hard links:

  ```sh
  oc-rsync -a --link-dest="../prev" --compare-dest="../base" "./src/" "./snapshot/"
  ```

- Tune delta block size:

  ```sh
  oc-rsync -B 65536 "./src" "remote:/dst"
  ```

- Emit JSON-formatted logs:

  ```sh
  oc-rsync --log-format json -v ./src ./dst
  ```

- Change ownership during transfer (requires root):

  ```sh
  sudo oc-rsync --chown=0:0 ./src/ remote:/dst/
  ```

- Show version:

  ```sh
  oc-rsync --version
  ```

### Trailing slash semantics

Just like `rsync`, adding a trailing slash to the source path changes what is
copied:

- `oc-rsync "src/" "dest/"` copies the *contents* of `src` into `dest`.
- `oc-rsync "src" "dest/"` creates a `dest/src` directory containing the original
  files.
- `oc-rsync "remote:/src/" "local/"` pulls the contents of `/src` from the remote
  host into `local`.

## Options

The table below mirrors the full `rsync(1)` flag set. Defaults show the
behavior of stock rsync; `off` means the flag is disabled unless specified.
Each entry links to the corresponding row in
[`feature_matrix.md`](feature_matrix.md) for implementation status and notes.

| Short | Long | Default | Interactions | Matrix |
|-------|------|---------|-------------|--------|
| `-8` | `--8-bit-output` | off |  | [matrix](feature_matrix.md#--8-bit-output) |
| `-A` | `--acls` | off | requires `acl` feature | [matrix](feature_matrix.md#--acls) |
|  | `--address` | 0.0.0.0 |  | [matrix](feature_matrix.md#--address) |
|  | `--append` | off |  | [matrix](feature_matrix.md#--append) |
|  | `--append-verify` | off |  | [matrix](feature_matrix.md#--append-verify) |
| `-a` | `--archive` | off |  | [matrix](feature_matrix.md#--archive) |
| `-U` | `--atimes` | off |  | [matrix](feature_matrix.md#--atimes) |
| `-b` | `--backup` | off | uses `~` suffix without `--backup-dir` | [matrix](feature_matrix.md#--backup) |
|  | `--backup-dir` | off | implies `--backup` | [matrix](feature_matrix.md#--backup-dir) |
| `-B` | `--block-size` | auto | controls delta block size | [matrix](feature_matrix.md#--block-size) |
|  | `--blocking-io` | off |  | [matrix](feature_matrix.md#--blocking-io) |
|  | `--bwlimit` | off |  | [matrix](feature_matrix.md#--bwlimit) |
|  | `--cc` | off | alias for `--checksum-choice` | [matrix](feature_matrix.md#--cc) |
| `-c` | `--checksum` | off | strong hashes: MD5 (default), SHA-1 | [matrix](feature_matrix.md#--checksum) |
|  | `--checksum-choice` | off |  | [matrix](feature_matrix.md#--checksum-choice) |
|  | `--checksum-seed` | off |  | [matrix](feature_matrix.md#--checksum-seed) |
|  | `--chmod` | off |  | [matrix](feature_matrix.md#--chmod) |
|  | `--chown` | off |  | [matrix](feature_matrix.md#--chown) |
|  | `--compare-dest` | off |  | [matrix](feature_matrix.md#--compare-dest) |
| `-z` | `--compress` | off | negotiates zstd or zlib | [matrix](feature_matrix.md#--compress) |
|  | `--compress-choice` | auto | supports zstd and zlib | [matrix](feature_matrix.md#--compress-choice) |
|  | `--compress-level` | auto | applies to zlib or zstd | [matrix](feature_matrix.md#--compress-level) |
|  | `--zc` | off | alias for `--compress-choice` | [matrix](feature_matrix.md#--zc) |
|  | `--zl` | off | alias for `--compress-level` | [matrix](feature_matrix.md#--zl) |
|  | `--contimeout` | off |  | [matrix](feature_matrix.md#--contimeout) |
|  | `--copy-as` | off |  | [matrix](feature_matrix.md#--copy-as) |
|  | `--copy-dest` | off |  | [matrix](feature_matrix.md#--copy-dest) |
|  | `--copy-devices` | off |  | [matrix](feature_matrix.md#--copy-devices) |
| `-k` | `--copy-dirlinks` | off |  | [matrix](feature_matrix.md#--copy-dirlinks) |
| `-L` | `--copy-links` | off |  | [matrix](feature_matrix.md#--copy-links) |
|  | `--copy-unsafe-links` | off |  | [matrix](feature_matrix.md#--copy-unsafe-links) |
| `-N` | `--crtimes` | off |  | [matrix](feature_matrix.md#--crtimes) |
| `-C` | `--cvs-exclude` | off |  | [matrix](feature_matrix.md#--cvs-exclude) |
|  | `--daemon` | off |  | [matrix](feature_matrix.md#--daemon) |
|  | `--debug` | off |  | [matrix](feature_matrix.md#--debug) |
|  | `--del` | off | alias for `--delete-during` | [matrix](feature_matrix.md#--del) |
|  | `--delay-updates` | off |  | [matrix](feature_matrix.md#--delay-updates) |
|  | `--delete` | off |  | [matrix](feature_matrix.md#--delete) |
|  | `--delete-after` | off |  | [matrix](feature_matrix.md#--delete-after) |
|  | `--delete-before` | off |  | [matrix](feature_matrix.md#--delete-before) |
|  | `--delete-delay` | off |  | [matrix](feature_matrix.md#--delete-delay) |
|  | `--delete-during` | off |  | [matrix](feature_matrix.md#--delete-during) |
|  | `--delete-excluded` | off |  | [matrix](feature_matrix.md#--delete-excluded) |
|  | `--delete-missing-args` | off |  | [matrix](feature_matrix.md#--delete-missing-args) |
|  | `--devices` | off |  | [matrix](feature_matrix.md#--devices) |
| `-d` | `--dirs` | off |  | [matrix](feature_matrix.md#--dirs) |
| `-n` | `--dry-run` | off |  | [matrix](feature_matrix.md#--dry-run) |
|  | `--early-input` | off |  | [matrix](feature_matrix.md#--early-input) |
|  | `--exclude` | off |  | [matrix](feature_matrix.md#--exclude) |
|  | `--exclude-from` | off |  | [matrix](feature_matrix.md#--exclude-from) |
| `-E` | `--executability` | off |  | [matrix](feature_matrix.md#--executability) |
|  | `--existing` | off |  | [matrix](feature_matrix.md#--existing) |
|  | `--fake-super` | off |  | [matrix](feature_matrix.md#--fake-super) |
|  | `--files-from` | off |  | [matrix](feature_matrix.md#--files-from) |
| `-f` | `--filter` | off |  | [matrix](feature_matrix.md#--filter) |
|  | `--force` | off |  | [matrix](feature_matrix.md#--force) |
| `-0` | `--from0` | off |  | [matrix](feature_matrix.md#--from0) |
|  | `--fsync` | off |  | [matrix](feature_matrix.md#--fsync) |
| `-y` | `--fuzzy` | off |  | [matrix](feature_matrix.md#--fuzzy) |
| `-g` | `--group` | off |  | [matrix](feature_matrix.md#--group) |
|  | `--groupmap` | off |  | [matrix](feature_matrix.md#--groupmap) |
| `-H` | `--hard-links` | off |  | [matrix](feature_matrix.md#--hard-links) |
| `-h (*)` | `--help` | off |  | [matrix](feature_matrix.md#--help) |
|  | `--human-readable` | off |  | [matrix](feature_matrix.md#--human-readable) |
|  | `--iconv` | off | request charset conversion of filenames | [matrix](feature_matrix.md#--iconv) |
|  | `--ignore-errors` | off |  | [matrix](feature_matrix.md#--ignore-errors) |
|  | `--ignore-existing` | off |  | [matrix](feature_matrix.md#--ignore-existing) |
|  | `--ignore-missing-args` | off |  | [matrix](feature_matrix.md#--ignore-missing-args) |
| `-I` | `--ignore-times` | off |  | [matrix](feature_matrix.md#--ignore-times) |
|  | `--include` | off |  | [matrix](feature_matrix.md#--include) |
|  | `--include-from` | off |  | [matrix](feature_matrix.md#--include-from) |
|  | `--info` | off |  | [matrix](feature_matrix.md#--info) |
|  | `--inplace` | off |  | [matrix](feature_matrix.md#--inplace) |
| `-4` | `--ipv4` | off |  | [matrix](feature_matrix.md#--ipv4) |
| `-6` | `--ipv6` | off |  | [matrix](feature_matrix.md#--ipv6) |
| `-i` | `--itemize-changes` | off |  | [matrix](feature_matrix.md#--itemize-changes) |
| `-K` | `--keep-dirlinks` | off |  | [matrix](feature_matrix.md#--keep-dirlinks) |
|  | `--link-dest` | off |  | [matrix](feature_matrix.md#--link-dest) |
| `-l` | `--links` | off |  | [matrix](feature_matrix.md#--links) |
|  | `--list-only` | off |  | [matrix](feature_matrix.md#--list-only) |
|  | `--log-file` | off |  | [matrix](feature_matrix.md#--log-file) |
|  | `--log-file-format` | off |  | [matrix](feature_matrix.md#--log-file-format) |
|  | `--max-alloc` | off |  | [matrix](feature_matrix.md#--max-alloc) |
|  | `--max-delete` | off |  | [matrix](feature_matrix.md#--max-delete) |
|  | `--max-size` | off |  | [matrix](feature_matrix.md#--max-size) |
|  | `--min-size` | off |  | [matrix](feature_matrix.md#--min-size) |
|  | `--mkpath` | off |  | [matrix](feature_matrix.md#--mkpath) |
| `-@` | `--modify-window` | off |  | [matrix](feature_matrix.md#--modify-window) |
|  | `--munge-links` | off |  | [matrix](feature_matrix.md#--munge-links) |
|  | `--no-D` | off | alias for `--no-devices --no-specials` | [matrix](feature_matrix.md#--no-d) |
|  | `--no-OPTION` | off |  | [matrix](feature_matrix.md#--no-option) |
|  | `--no-implied-dirs` | off | skip creating implicit ancestor directories | [matrix](feature_matrix.md#--no-implied-dirs) |
|  | `--no-motd` | off |  | [matrix](feature_matrix.md#--no-motd) |
|  | `--numeric-ids` | off |  | [matrix](feature_matrix.md#--numeric-ids) |
|  | `--old-args` | off |  | [matrix](feature_matrix.md#--old-args) |
|  | `--old-d` | off | alias for `--old-dirs` | [matrix](feature_matrix.md#--old-d) |
|  | `--old-dirs` | off |  | [matrix](feature_matrix.md#--old-dirs) |
| `-O` | `--omit-dir-times` | off |  | [matrix](feature_matrix.md#--omit-dir-times) |
| `-J` | `--omit-link-times` | off |  | [matrix](feature_matrix.md#--omit-link-times) |
| `-x` | `--one-file-system` | off |  | [matrix](feature_matrix.md#--one-file-system) |
|  | `--only-write-batch` | off |  | [matrix](feature_matrix.md#--only-write-batch) |
|  | `--open-noatime` | off |  | [matrix](feature_matrix.md#--open-noatime) |
|  | `--out-format` | off |  | [matrix](feature_matrix.md#--out-format) |
|  | `--outbuf` | off |  | [matrix](feature_matrix.md#--outbuf) |
| `-o` | `--owner` | off |  | [matrix](feature_matrix.md#--owner) |
|  | `--partial` | off |  | [matrix](feature_matrix.md#--partial) |
|  | `--partial-dir` | off |  | [matrix](feature_matrix.md#--partial-dir) |
|  | `--password-file` | — |  | [matrix](feature_matrix.md#--password-file) |
| `-p` | `--perms` | off |  | [matrix](feature_matrix.md#--perms) |
|  | `--port` | 873 |  | [matrix](feature_matrix.md#--port) |
|  | `--preallocate` | off |  | [matrix](feature_matrix.md#--preallocate) |
|  | `--progress` | off |  | [matrix](feature_matrix.md#--progress) |
|  | `--protocol` | off |  | [matrix](feature_matrix.md#--protocol) |
| `-m` | `--prune-empty-dirs` | off |  | [matrix](feature_matrix.md#--prune-empty-dirs) |
| `-q` | `--quiet` | off |  | [matrix](feature_matrix.md#--quiet) |
|  | `--read-batch` | off |  | [matrix](feature_matrix.md#--read-batch) |
| `-r` | `--recursive` | off |  | [matrix](feature_matrix.md#--recursive) |
| `-R` | `--relative` | off |  | [matrix](feature_matrix.md#--relative) |
| `-M` | `--remote-option` | off | forward option to remote side only; repeat for multiple | [matrix](feature_matrix.md#--remote-option) |
|  | `--remove-source-files` | off |  | [matrix](feature_matrix.md#--remove-source-files) |
| `-e` | `--rsh` | ssh | negotiation incomplete; lacks full command parsing and environment handshake | [matrix](feature_matrix.md#--rsh) |
|  | `--rsync-path` | — | requires `--rsh`; remote path negotiation incomplete | [matrix](feature_matrix.md#--rsync-path) |
|  | `--safe-links` | off |  | [matrix](feature_matrix.md#--safe-links) |
| `-s` | `--secluded-args` | off |  | [matrix](feature_matrix.md#--secluded-args) |
|  | `--secrets-file` | off |  | [matrix](feature_matrix.md#--secrets-file) |
|  | `--server` | off | negotiates protocol version and codecs | [matrix](feature_matrix.md#--server) |
|  | `--size-only` | off |  | [matrix](feature_matrix.md#--size-only) |
|  | `--skip-compress` | off |  | [matrix](feature_matrix.md#--skip-compress) |
|  | `--sockopts` | off | comma-separated socket options, e.g. `SO_KEEPALIVE` or `ip:ttl=64` | [matrix](feature_matrix.md#--sockopts) |
| `-S` | `--sparse` | off | creates holes for long zero runs | [matrix](feature_matrix.md#--sparse) |
|  | `--specials` | off |  | [matrix](feature_matrix.md#--specials) |
|  | `--stats` | off |  | [matrix](feature_matrix.md#--stats) |
|  | `--stderr` | off |  | [matrix](feature_matrix.md#--stderr) |
|  | `--stop-after` | off |  | [matrix](feature_matrix.md#--stop-after) |
|  | `--stop-at` | off |  | [matrix](feature_matrix.md#--stop-at) |
|  | `--suffix` | off |  | [matrix](feature_matrix.md#--suffix) |
|  | `--super` | off |  | [matrix](feature_matrix.md#--super) |
| `-T` | `--temp-dir` | off |  | [matrix](feature_matrix.md#--temp-dir) |
|  | `--timeout` | off | set idle and I/O timeout in seconds | [matrix](feature_matrix.md#--timeout) |
| `-t` | `--times` | off |  | [matrix](feature_matrix.md#--times) |
|  | `--trust-sender` | off |  | [matrix](feature_matrix.md#--trust-sender) |
| `-u` | `--update` | off |  | [matrix](feature_matrix.md#--update) |
|  | `--usermap` | off |  | [matrix](feature_matrix.md#--usermap) |
| `-v` | `--verbose` | off |  | [matrix](feature_matrix.md#--verbose) |
| `-V` | `--version` | off |  | [matrix](feature_matrix.md#--version) |
| `-W` | `--whole-file` | off |  | [matrix](feature_matrix.md#--whole-file) |
|  | `--write-batch` | off |  | [matrix](feature_matrix.md#--write-batch) |
|  | `--write-devices` | off |  | [matrix](feature_matrix.md#--write-devices) |
| `-X` | `--xattrs` | off | requires `xattr` feature | [matrix](feature_matrix.md#--xattrs) |

Implementation details for each flag live in the
[feature matrix](feature_matrix.md) and the
[CLI flag reference](cli/flags.md).

### Format escapes

`oc-rsync` understands a small set of escapes in `--out-format` and
`--log-file-format` strings:

| Escape | Meaning |
|--------|---------|
| `%n` | file name |
| `%L` | symlink target (prefixed by ` -> `) |
| `%i` | itemized change string |
| `%o` | operation (`send`, `recv`, `del.`) |
| `%%` | literal percent sign |
| `\\a` | bell |
| `\\b` | backspace |
| `\\e` | escape |
| `\\f` | form feed |
| `\\n` | newline |
| `\\r` | carriage return |
| `\\t` | tab |
| `\\v` | vertical tab |
| `\\` | backslash |
| `\\0NNN` | octal byte value |
| `\\xHH` | hex byte value |

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

```sh
oc-rsync -e 'RUST_LOG=debug ssh -p 2222 -o "StrictHostKeyChecking=no"' "src" "dst"
```

During the connection handshake `oc-rsync` also forwards any environment
variables from its own process whose names begin with `RSYNC_`, mirroring
`rsync`'s environment propagation behavior.

## Configuration precedence

1. Command-line flags
2. Environment variables (prefixed with `OC_RSYNC_`)
3. Configuration file provided via `--config`
4. Built-in defaults

No configuration file is loaded unless `--config` is supplied. Settings
specified earlier in the list override later sources.

## Filters

`oc-rsync` supports include and exclude rules using the same syntax as
`rsync`. Rules can be supplied on the command line with `--filter` or placed in
`.rsync-filter` files located throughout the source tree. When walking
directories the tool automatically loads any `.rsync-filter` files and merges
their rules with those from parent directories. Rules in deeper directories take
precedence over rules defined higher up.

### Example

```text
project/
├── .rsync-filter      # contains: - *.tmp
└── logs/
    ├── .rsync-filter  # contains: + keep.tmp
    ├── keep.tmp
    └── other.tmp
```

In this layout, `keep.tmp` is included because the rule in `logs/.rsync-filter`
overrides the root exclusion. The file `other.tmp` remains excluded.
