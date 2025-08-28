# Feature Matrix


This table tracks the implementation status of rsync 3.2.x command-line options.


| Option | Supported | Behavior Parity | Tests | Notes |
|-------|-----------|-----------------|-------|-------|
| `--verbose, -v` | ✅ |  |  |  |
| `--info=FLAGS` |  |  |  |  |
| `--debug=FLAGS` |  |  |  |  |
| `--stderr=e|a|c` |  |  |  |  |
| `--quiet, -q` | ✅ |  |  |  |
| `--no-motd` |  |  |  |  |
| `--checksum, -c` | ✅ |  |  |  |
| `--archive, -a` | ✅ |  |  |  |
| `--no-OPTION` |  |  |  |  |
| `--recursive, -r` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh)<br>[tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh)<br>[tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  |
| `--relative, -R` | ✅ |  | [tests/cli.rs](../tests/cli.rs) |  |
| `--no-implied-dirs` |  |  |  |  |
| `--backup, -b` |  |  |  |  |
| `--backup-dir=DIR` |  |  |  |  |
| `--suffix=SUFFIX` |  |  |  |  |
| `--update, -u` |  |  |  |  |
| `--inplace` |  |  |  |  |
| `--append` |  |  |  |  |
| `--append-verify` |  |  |  |  |
| `--dirs, -d` |  |  |  |  |
| `--old-dirs, --old-d` |  |  |  |  |
| `--mkpath` |  |  |  |  |
| `--links, -l` |  |  |  |  |
| `--copy-links, -L` |  |  |  |  |
| `--copy-unsafe-links` |  |  |  |  |
| `--safe-links` |  |  |  |  |
| `--munge-links` |  |  |  |  |
| `--copy-dirlinks, -k` |  |  |  |  |
| `--keep-dirlinks, -K` |  |  |  |  |
| `--hard-links, -H` |  |  |  |  |
| `--perms, -p` |  |  |  |  |
| `--executability, -E` |  |  |  |  |
| `--chmod=CHMOD` |  |  |  |  |
| `--acls, -A` |  |  |  |  |
| `--xattrs, -X` |  |  |  |  |
| `--owner, -o` | ✅ |  | [tests/cli.rs](../tests/cli.rs) |  |
| `--group, -g` | ✅ |  | [tests/cli.rs](../tests/cli.rs) |  |
| `--devices` |  |  |  |  |
| `--copy-devices` |  |  |  |  |
| `--write-devices` |  |  |  |  |
| `--specials` |  |  |  |  |
| `-D` |  |  |  |  |
| `--times, -t` |  |  |  |  |
| `--atimes, -U` |  |  |  |  |
| `--open-noatime` |  |  |  |  |
| `--crtimes, -N` |  |  |  |  |
| `--omit-dir-times, -O` |  |  |  |  |
| `--omit-link-times, -J` |  |  |  |  |
| `--super` |  |  |  |  |
| `--fake-super` |  |  |  |  |
| `--sparse, -S` | ✅ |  | [crates/engine/tests/attrs.rs](../crates/engine/tests/attrs.rs) |  |
| `--preallocate` |  |  |  |  |
| `--dry-run, -n` | ✅ |  |  |  |
| `--whole-file, -W` |  |  |  |  |
| `--checksum-choice=STR` |  |  |  |  |
| `--one-file-system, -x` |  |  |  |  |
| `--block-size=SIZE, -B` |  |  |  |  |
| `--rsh=COMMAND, -e` |  |  |  |  |
| `--rsync-path=PROGRAM` |  |  |  |  |
| `--existing` |  |  |  |  |
| `--ignore-existing` |  |  |  |  |
| `--remove-source-files` |  |  |  |  |
| `--del` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | |
| `--delete` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | |
| `--delete-before` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | |
| `--delete-during` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | |
| `--delete-delay` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | treated as `--delete-after` |
| `--delete-after` | ✅ | ✅ | [tests/golden/cli_parity/delete.sh](../tests/golden/cli_parity/delete.sh) | |
| `--delete-excluded` |  |  |  |  |
| `--ignore-missing-args` |  |  |  |  |
| `--delete-missing-args` |  |  |  |  |
| `--ignore-errors` |  |  |  |  |
| `--force` |  |  |  |  |
| `--max-delete=NUM` |  |  |  |  |
| `--max-size=SIZE` |  |  |  |  |
| `--min-size=SIZE` |  |  |  |  |
| `--max-alloc=SIZE` |  |  |  |  |
| `--partial` |  |  |  |  |
| `--partial-dir=DIR` |  |  |  |  |
| `--delay-updates` |  |  |  |  |
| `--prune-empty-dirs, -m` |  |  |  |  |
| `--numeric-ids` | ✅ |  | [tests/cli.rs](../tests/cli.rs) |  |
| `--usermap=STRING` |  |  |  |  |
| `--groupmap=STRING` |  |  |  |  |
| `--chown=USER:GROUP` |  |  |  |  |
| `--timeout=SECONDS` |  |  |  |  |
| `--contimeout=SECONDS` |  |  |  |  |
| `--ignore-times, -I` |  |  |  |  |
| `--size-only` |  |  |  |  |
| `--modify-window=NUM, -@` |  |  |  |  |
| `--temp-dir=DIR, -T` |  |  |  |  |
| `--fuzzy, -y` |  |  |  |  |
| `--compare-dest=DIR` |  |  |  |  |
| `--copy-dest=DIR` |  |  |  |  |
| `--link-dest=DIR` |  |  |  |  |
| `--compress, -z` | ✅ | ✅ | [tests/golden/cli_parity/compression.sh](../tests/golden/cli_parity/compression.sh) |  |
| `--compress-choice=STR` |  |  |  |  |
| `--compress-level=NUM` | ✅ |  |  |  |
| `--skip-compress=LIST` |  |  |  |  |
| `--cvs-exclude, -C` |  |  |  |  |
| `--filter=RULE, -f` | ✅ | ✅ | [tests/golden/cli_parity/selection.sh](../tests/golden/cli_parity/selection.sh) |  |
| `-F` |  |  |  |  |
| `--exclude=PATTERN` |  |  |  |  |
| `--exclude-from=FILE` |  |  |  |  |
| `--include=PATTERN` |  |  |  |  |
| `--include-from=FILE` |  |  |  |  |
| `--files-from=FILE` |  |  |  |  |
| `--from0, -0` |  |  |  |  |
| `--old-args` |  |  |  |  |
| `--secluded-args, -s` |  |  |  |  |
| `--trust-sender` |  |  |  |  |
| `--copy-as=USER[:GROUP]` |  |  |  |  |
| `--address=ADDRESS` |  |  |  |  |
| `--port=PORT` | ✅ |  |  |  |
| `--sockopts=OPTIONS` |  |  |  |  |
| `--blocking-io` |  |  |  |  |
| `--outbuf=N|L|B` |  |  |  |  |
| `--stats` | ✅ |  |  |  |
| `--8-bit-output, -8` |  |  |  |  |
| `--human-readable, -h` |  |  |  |  |
| `--progress` |  |  |  |  |
| `-P` | ✅ |  | [tests/cli.rs](../tests/cli.rs) | alias for --partial and --progress |
| `--itemize-changes, -i` |  |  |  |  |
| `--remote-option=OPT, -M` |  |  |  |  |
| `--out-format=FORMAT` |  |  |  |  |
| `--log-file=FILE` |  |  |  |  |
| `--log-file-format=FMT` |  |  |  |  |
| `--password-file=FILE` |  |  |  |  |
| `--early-input=FILE` |  |  |  |  |
| `--list-only` |  |  |  |  |
| `--bwlimit=RATE` |  |  |  |  |
| `--stop-after=MINS` |  |  |  |  |
| `--stop-at=y-m-dTh:m` |  |  |  |  |
| `--fsync` |  |  |  |  |
| `--write-batch=FILE` |  |  |  |  |
| `--only-write-batch=FILE` |  |  |  |  |
| `--read-batch=FILE` |  |  |  |  |
| `--protocol=NUM` |  |  |  |  |
| `--iconv=CONVERT_SPEC` |  |  |  |  |
| `--checksum-seed=NUM` |  |  |  |  |
| `--ipv4, -4` |  |  |  |  |
| `--ipv6, -6` |  |  |  |  |
| `--version, -V` |  |  |  |  |
| `--help, -h (*)` |  |  |  |  |
| `--daemon` | ✅ |  | [tests/daemon.rs](../tests/daemon.rs) |  |
| `--config=FILE` | ✅ |  |  |  |
| `--dparam=OVERRIDE, -M` |  |  |  |  |
| `--no-detach` |  |  |  |  |
| `--help, -h` |  |  |  |  |