#compdef rsync-rs

autoload -U is-at-least

_rsync-rs() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--compress-choice=[choose the compression algorithm (aka --zc)]:STR:_default' \
'--zc=[choose the compression algorithm (aka --zc)]:STR:_default' \
'--compress-level=[explicitly set compression level]:NUM:_default' \
'--zl=[explicitly set compression level]:NUM:_default' \
'--partial-dir=[put a partially transferred file into DIR]:DIR:_files' \
'--bwlimit=[throttle I/O bandwidth to RATE bytes per second]:RATE:_default' \
'--link-dest=[hardlink to files in DIR when unchanged]:DIR:_files' \
'--copy-dest=[copy files from DIR when unchanged]:DIR:_files' \
'--compare-dest=[skip files that match in DIR]:DIR:_files' \
'--config=[supply a custom configuration file]:FILE:_files' \
'--known-hosts=[path to SSH known hosts file]:FILE:_files' \
'--password-file=[read daemon-access password from FILE]:FILE:_files' \
'-e+[specify the remote shell to use]:COMMAND:_default' \
'--rsh=[specify the remote shell to use]:COMMAND:_default' \
'--rsync-path=[specify the rsync to run on remote machine]:PATH:_files' \
'*-f+[filter rules provided directly]:RULE:_default' \
'*--filter=[filter rules provided directly]:RULE:_default' \
'*--filter-file=[files containing filter rules]:FILE:_files' \
'*--include=[include files matching PATTERN]:PATTERN:_default' \
'*--exclude=[exclude files matching PATTERN]:PATTERN:_default' \
'*--include-from=[read include patterns from FILE]:FILE:_files' \
'*--exclude-from=[read exclude patterns from FILE]:FILE:_files' \
'*--files-from=[read list of files from FILE]:FILE:_files' \
'--local[perform a local sync]' \
'-a[archive mode]' \
'--archive[archive mode]' \
'-r[copy directories recursively]' \
'--recursive[copy directories recursively]' \
'-R[use relative path names]' \
'--relative[use relative path names]' \
'-n[perform a trial run with no changes made]' \
'--dry-run[perform a trial run with no changes made]' \
'-S[turn sequences of nulls into sparse blocks and preserve existing holes (requires filesystem support)]' \
'--sparse[turn sequences of nulls into sparse blocks and preserve existing holes (requires filesystem support)]' \
'*-v[increase logging verbosity]' \
'*--verbose[increase logging verbosity]' \
'-q[suppress non-error messages]' \
'--quiet[suppress non-error messages]' \
'--no-motd[suppress daemon-mode MOTD]' \
'--delete[remove extraneous files from the destination]' \
'--delete-before[receiver deletes before transfer, not during]' \
'--delete-during[receiver deletes during the transfer]' \
'--delete-after[receiver deletes after transfer, not during]' \
'--delete-delay[find deletions during, delete after]' \
'--delete-excluded[also delete excluded files from destination]' \
'-c[use full checksums to determine file changes]' \
'--checksum[use full checksums to determine file changes]' \
'--perms[preserve permissions]' \
'--times[preserve modification times]' \
'-U[preserve access times]' \
'--atimes[preserve access times]' \
'-N[preserve create times]' \
'--crtimes[preserve create times]' \
'--owner[preserve owner]' \
'--group[preserve group]' \
'--links[copy symlinks as symlinks]' \
'--hard-links[preserve hard links]' \
'--devices[preserve device files]' \
'--specials[preserve special files]' \
'-z[compress file data during the transfer (zlib by default, negotiates zstd when supported)]' \
'--compress[compress file data during the transfer (zlib by default, negotiates zstd when supported)]' \
'--modern[enable BLAKE3 checksums (zstd is negotiated automatically)]' \
'--partial[keep partially transferred files]' \
'--progress[show progress during transfer]' \
'-P[keep partially transferred files and show progress]' \
'--append[append data onto shorter files]' \
'--append-verify[--append with old data verification]' \
'-I[update destination files in-place]' \
'--inplace[update destination files in-place]' \
'--numeric-ids[don'\''t map uid/gid values by user/group name]' \
'--stats[display transfer statistics on completion]' \
'--no-host-key-checking[disable strict host key checking (not recommended)]' \
'--server[run in server mode (internal use)]' \
'--sender[run in sender mode (internal use)]' \
'*-F[shorthand for per-directory filter files]' \
'--from0[treat file lists as null-separated]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
':src -- source path or HOST\:PATH:_default' \
':dst -- destination path or HOST\:PATH:_default' \
&& ret=0
}

(( $+functions[_rsync-rs_commands] )) ||
_rsync-rs_commands() {
    local commands; commands=()
    _describe -t commands 'rsync-rs commands' commands "$@"
}

if [ "$funcstack[1]" = "_rsync-rs" ]; then
    _rsync-rs "$@"
else
    compdef _rsync-rs rsync-rs
fi
