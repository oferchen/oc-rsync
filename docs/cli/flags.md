# rsync Flags

## Attributes

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
| -A | --acls | preserve ACLs (implies --perms) | yes | requires `acl` feature | no |
| -U | --atimes | preserve access (use) times | yes |  | no |
|  | --chmod=CHMOD | affect file and/or directory permissions | no |  | no |
|  | --chown=USER:GROUP | simple username/groupname mapping | no |  | no |
|  | --copy-devices | copy device contents as a regular file | no |  | no |
| -N | --crtimes | preserve create times (newness) | yes |  | no |
|  | --devices | preserve device files (super-user only) | yes |  | no |
| -E | --executability | preserve executability | no |  | no |
|  | --fake-super | store/recover privileged attrs using xattrs | no |  | no |
|  | --group | preserve group | yes |  | no |
|  | --groupmap=STRING | custom groupname mapping | no |  | no |
|  | --numeric-ids | don't map uid/gid values by user/group name | yes |  | no |
| -O | --omit-dir-times | omit directories from --times | no |  | no |
| -J | --omit-link-times | omit symlinks from --times | no |  | no |
|  | --open-noatime | avoid changing the atime on opened files | no |  | no |
|  | --owner | preserve owner (super-user only) | yes |  | no |
|  | --perms | preserve permissions | yes |  | no |
|  | --specials | preserve special files | yes |  | no |
|  | --super | receiver attempts super-user activities | no |  | no |
|  | --times | preserve modification times | yes |  | no |
|  | --usermap=STRING | custom username mapping | no |  | no |
|  | --write-devices | write to devices as files (implies --inplace) | no |  | no |
| -X | --xattrs | preserve extended attributes | yes | requires `xattr` feature | no |

## Compression

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
| -z | --compress | compress file data during the transfer | yes |  | no |
|  | --compress-choice=STR | choose the compression algorithm (aka --zc) | yes |  | no |
|  | --compress-level=NUM | explicitly set compression level (aka --zl) | yes |  | no |
|  | --modern | enable zstd compression and BLAKE3 checksums | yes | rsync has no equivalent | yes |
|  | --skip-compress=LIST | skip compressing files with suffix in LIST | no |  | no |

## Daemon

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
|  | --daemon | run as an rsync daemon | yes |  | no |
| -M | --dparam=OVERRIDE | override global daemon config parameter | no |  | no |
|  | --no-detach | do not detach from the parent | no |  | no |
|  | --secrets-file=FILE | path to secrets file for authentication | yes |  | no |

## Delete

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
|  | --del | an alias for --delete-during | no |  | no |
|  | --delete | delete extraneous files from dest dirs | yes |  | no |
|  | --delete-after | receiver deletes after transfer, not during | no |  | no |
|  | --delete-before | receiver deletes before xfer, not during | no |  | no |
|  | --delete-delay | find deletions during, delete after | no |  | no |
|  | --delete-during | receiver deletes during the transfer | no |  | no |
|  | --delete-excluded | also delete excluded files from dest dirs | no |  | no |
|  | --delete-missing-args | delete missing source args from destination | no |  | no |
|  | --force | force deletion of dirs even if not empty | no |  | no |
|  | --ignore-errors | delete even if there are I/O errors | no |  | no |
|  | --ignore-missing-args | ignore missing source args without error | no |  | no |
|  | --max-delete=NUM | don't delete more than NUM files | no |  | no |

## Filter

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
| -C | --cvs-exclude | auto-ignore files in the same way CVS does | no |  | no |
|  | --exclude-from=FILE | read exclude patterns from FILE | no |  | no |
|  | --exclude=PATTERN | exclude files matching PATTERN | no |  | no |
|  | --files-from=FILE | read list of source-file names from FILE | no |  | no |
| -f | --filter=RULE | add a file-filtering RULE | yes | Supports --filter-file | no |
| -0 | --from0 | all *-from/filter files are delimited by 0s | no |  | no |
|  | --include-from=FILE | read include patterns from FILE | no |  | no |
|  | --include=PATTERN | don't exclude files matching PATTERN | no |  | no |
|  | --old-args | disable the modern arg-protection idiom | no |  | no |
| -s | --secluded-args | use the protocol to safely send the args | no |  | no |
|  | --trust-sender | trust the remote sender's file list | no |  | no |
| -F |  | same as --filter='dir-merge /.rsync-filter' repeated: --filter='- .rsync-filter' | no |  | no |

## Misc

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
|  | --config=FILE | specify alternate rsyncd.conf file | yes | Used for client config file | yes |
| -n | --dry-run | perform a trial run with no changes made | yes |  | no |
|  | --fsync | fsync every written file | no |  | no |
|  | --only-write-batch=FILE | like --write-batch but w/o updating dest | no |  | no |
|  | --read-batch=FILE | read a batched update from FILE | no |  | no |
|  | --stop-after=MINS | Stop rsync after MINS minutes have elapsed | no |  | no |
|  | --stop-at=y-m-dTh:m | Stop rsync at the specified point in time | no |  | no |
|  | --write-batch=FILE | write a batched update to FILE | no |  | no |
| -D |  | same as --devices --specials | no |  | no |
| -P |  | same as --partial --progress | yes |  | no |

## Network

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
|  | --address=ADDRESS | bind address for outgoing socket to daemon | no |  | no |
|  | --blocking-io | use blocking I/O for the remote shell | no |  | no |
|  | --bwlimit=RATE | limit socket I/O bandwidth | no |  | no |
|  | --copy-as=USER[:GROUP] | specify user & optional group for the copy | no |  | no |
|  | --early-input=FILE | use FILE for daemon's early exec input | no |  | no |
|  | --iconv=CONVERT_SPEC | request charset conversion of filenames | no |  | no |
| -4 | --ipv4 | prefer IPv4 | no |  | no |
| -6 | --ipv6 | prefer IPv6 | no |  | no |
|  | --outbuf=N\|L\|B | set out buffering to None, Line, or Block | no |  | no |
|  | --password-file=FILE | read daemon-access password from FILE | yes |  | no |
|  | --port=PORT | specify double-colon alternate port number | yes |  | no |
|  | --protocol=NUM | force an older protocol version to be used | no |  | no |
| -M | --remote-option=OPT | send OPTION to the remote side only | no |  | no |
| -e | --rsh=COMMAND | specify the remote shell to use | no |  | no |
|  | --rsync-path=PROGRAM | specify the rsync to run on remote machine | no |  | no |
|  | --sockopts=OPTIONS | specify custom TCP options | no |  | no |

## Output

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
| -8 | --8-bit-output | leave high-bit chars unescaped in output | no |  | no |
|  | --debug=FLAGS | fine-grained debug verbosity | no |  | no |
| -h (*) | --help | show this help (* -h is help only on its own) Rsync can also be run as a daemon, in which case the following options are accepted: | no |  | no |
| -h | --help | show this help (when used with --daemon) | no |  | no |
| -h | --human-readable | output numbers in a human-readable format | no |  | no |
|  | --info=FLAGS | fine-grained informational verbosity | no |  | no |
| -i | --itemize-changes | output a change-summary for all updates | no |  | no |
|  | --list-only | list the files instead of copying them | no |  | no |
|  | --log-file-format=FMT | log updates using the specified FMT | no |  | no |
|  | --log-file=FILE | log what we're doing to the specified FILE | no |  | no |
|  | --no-motd | suppress daemon-mode MOTD | no |  | no |
|  | --out-format=FORMAT | output updates using the specified FORMAT | no |  | no |
|  | --progress | show progress during transfer | yes |  | no |
| -q | --quiet | suppress non-error messages | yes |  | no |
|  | --stats | give some file-transfer stats | yes |  | no |
|  | --stderr=e\|a\|c | change stderr output mode (default: errors) | no |  | no |
| -v | --verbose | increase verbosity | yes |  | no |
| -V | --version | print the version + other info and exit | no |  | no |

## Selection

| short | long | summary | implemented? | notes | enhanced? |
| --- | --- | --- | :---: | --- | :---: |
|  | --append | append data onto shorter files | no |  | no |
|  | --append-verify | --append w/old data in file checksum | no |  | no |
| -a | --archive | archive mode is -rlptgoD (no -A,-X,-U,-N,-H) | yes |  | no |
| -b | --backup | make backups (see --suffix & --backup-dir) | yes |  | no |
|  | --backup-dir=DIR | make backups into hierarchy based in DIR | yes |  | no |
| -B | --block-size=SIZE | force a fixed checksum block-size | no |  | no |
| -c | --checksum | skip based on checksum, not mod-time & size | no | Parsed but not implemented | no |
|  | --checksum-choice=STR | choose the checksum algorithm (aka --cc) | yes |  | no |
|  | --checksum-seed=NUM | set block/file checksum seed (advanced) | no |  | no |
|  | --compare-dest=DIR | also compare destination files relative to DIR | yes |  | no |
|  | --contimeout=SECONDS | set daemon connection timeout in seconds | yes |  | no |
|  | --copy-dest=DIR | ... and include copies of unchanged files | yes |  | no |
| -k | --copy-dirlinks | transform symlink to dir into referent dir | no |  | no |
| -L | --copy-links | transform symlink into referent file/dir | no |  | no |
|  | --copy-unsafe-links | only "unsafe" symlinks are transformed | no |  | no |
|  | --delay-updates | put all updated files into place at end | no |  | no |
| -d | --dirs | transfer directories without recursing | no |  | no |
|  | --existing | skip creating new files on receiver | no |  | no |
| -y | --fuzzy | find similar file for basis if no dest file | no |  | no |
|  | --hard-links | preserve hard links | yes |  | no |
|  | --ignore-existing | skip updating files that exist on receiver | no |  | no |
| -I | --ignore-times | don't skip files that match size and time | no |  | no |
|  | --inplace | update destination files in-place | no |  | no |
| -K | --keep-dirlinks | treat symlinked dir on receiver as dir | no |  | no |
|  | --link-dest=DIR | hardlink to files in DIR when unchanged | yes |  | no |
|  | --links | copy symlinks as symlinks | yes |  | no |
|  | --max-alloc=SIZE | change a limit relating to memory alloc | no |  | no |
|  | --max-size=SIZE | don't transfer any file larger than SIZE | no |  | no |
|  | --min-size=SIZE | don't transfer any file smaller than SIZE | no |  | no |
|  | --mkpath | create destination's missing path components | no |  | no |
| -@ | --modify-window=NUM | set the accuracy for mod-time comparisons | no |  | no |
|  | --munge-links | munge symlinks to make them safe & unusable | no |  | no |
|  | --no-OPTION | turn off an implied OPTION (e.g. --no-D) | no |  | no |
|  | --no-implied-dirs | don't send implied dirs with --relative | no |  | no |
| --old-d | --old-dirs | works like --dirs when talking to old rsync | no |  | no |
| -x | --one-file-system | don't cross filesystem boundaries | no |  | no |
|  | --partial | keep partially transferred files | yes |  | no |
|  | --partial-dir=DIR | put a partially transferred file into DIR | yes |  | no |
|  | --preallocate | allocate dest files before writing them | no |  | no |
| -m | --prune-empty-dirs | prune empty directory chains from file-list | no |  | no |
| -r | --recursive | recurse into directories | yes |  | no |
| -R | --relative | use relative path names | yes |  | no |
|  | --remove-source-files | sender removes synchronized files (non-dir) | no |  | no |
|  | --safe-links | ignore symlinks that point outside the tree | no |  | no |
|  | --size-only | skip files that match in size | no |  | no |
| -S | --sparse | turn sequences of nulls into sparse blocks (requires filesystem support) | yes |  | no |
|  | --suffix=SUFFIX | backup suffix (default ~ w/o --backup-dir) | no |  | no |
| -T | --temp-dir=DIR | create temporary files in directory DIR | no |  | no |
|  | --timeout=SECONDS | set I/O timeout in seconds | yes |  | no |
| -u | --update | skip files that are newer on the receiver | no |  | no |
| -W | --whole-file | copy files whole (w/o delta-xfer algorithm) | no |  | no |