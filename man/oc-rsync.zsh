#compdef oc-rsync

autoload -U is-at-least

_oc-rsync() {
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
'--backup-dir=[]:DIR:_files' \
'--checksum-choice=[]:STR:_default' \
'--cc=[]:STR:_default' \
'--checksum-seed=[set block/file checksum seed (advanced)]:NUM:_default' \
'*--chmod=[]:CHMOD:_default' \
'--chown=[]:USER:GROUP:_default' \
'--compress-choice=[]:STR:_default' \
'--zc=[]:STR:_default' \
'--compress-level=[]:NUM:_default' \
'--zl=[]:NUM:_default' \
'*--skip-compress=[]:LIST:_default' \
'--modern-compress=[]:MODERN_COMPRESS:(auto zstd lz4)' \
'--modern-hash=[]:MODERN_HASH:()' \
'--modern-cdc=[]:MODERN_CDC:(fastcdc off)' \
'--partial-dir=[]:DIR:_files' \
'-T+[]:DIR:_files' \
'--temp-dir=[]:DIR:_files' \
'--bwlimit=[]:RATE:_default' \
'--timeout=[]:SECONDS:_default' \
'--contimeout=[]:SECONDS:_default' \
'--protocol=[force an older protocol version]:VER:_default' \
'--port=[]:PORT:_default' \
'-B+[]:SIZE:_default' \
'--block-size=[]:SIZE:_default' \
'--link-dest=[]:DIR:_files' \
'--copy-dest=[]:DIR:_files' \
'--compare-dest=[]:DIR:_files' \
'--config=[]:FILE:_files' \
'--known-hosts=[]:FILE:_files' \
'--password-file=[]:FILE:_files' \
'--early-input=[]:FILE:_files' \
'-e+[]:COMMAND:_default' \
'--rsh=[]:COMMAND:_default' \
'--rsync-path=[]:PATH:_default' \
'*-f+[]:RULE:_default' \
'*--filter=[]:RULE:_default' \
'*--filter-file=[]:FILE:_files' \
'*--include=[]:PATTERN:_default' \
'*--exclude=[]:PATTERN:_default' \
'*--include-from=[]:FILE:_files' \
'*--exclude-from=[]:FILE:_files' \
'*--files-from=[]:FILE:_files' \
'--local[]' \
'-a[]' \
'--archive[]' \
'-r[]' \
'--recursive[]' \
'-d[]' \
'--dirs[]' \
'-R[]' \
'--relative[]' \
'-n[]' \
'--dry-run[]' \
'--list-only[]' \
'-S[]' \
'--sparse[]' \
'-u[]' \
'--update[]' \
'--ignore-existing[]' \
'--size-only[]' \
'-I[]' \
'--ignore-times[]' \
'*-v[]' \
'*--verbose[]' \
'--human-readable[]' \
'-q[]' \
'--quiet[]' \
'--no-motd[]' \
'-8[]' \
'--8-bit-output[]' \
'-i[output a change-summary for all updates]' \
'--itemize-changes[output a change-summary for all updates]' \
'--delete[]' \
'--delete-before[]' \
'--delete-during[]' \
'--delete-after[]' \
'--delete-delay[]' \
'--delete-excluded[]' \
'-b[]' \
'--backup[]' \
'-c[]' \
'--checksum[]' \
'--perms[]' \
'-E[]' \
'--executability[]' \
'--times[]' \
'-U[]' \
'--atimes[]' \
'-N[]' \
'--crtimes[]' \
'-O[]' \
'--omit-dir-times[]' \
'-J[]' \
'--omit-link-times[]' \
'--owner[]' \
'--group[]' \
'--links[]' \
'-L[]' \
'--copy-links[]' \
'-k[]' \
'--copy-dirlinks[]' \
'--copy-unsafe-links[]' \
'--safe-links[]' \
'--hard-links[]' \
'--devices[]' \
'--specials[]' \
'-z[]' \
'--compress[]' \
'--modern[Enable modern compression (zstd or lz4) and BLAKE3 checksums (requires \`blake3\` feature)]' \
'--partial[]' \
'--progress[]' \
'--blocking-io[]' \
'-P[]' \
'--append[]' \
'--append-verify[]' \
'--inplace[]' \
'(-6 --ipv6)-4[]' \
'(-6 --ipv6)--ipv4[]' \
'(-4 --ipv4)-6[]' \
'(-4 --ipv4)--ipv6[]' \
'-W[]' \
'--whole-file[]' \
'--no-whole-file[]' \
'--numeric-ids[]' \
'--stats[]' \
'--no-host-key-checking[]' \
'--server[]' \
'--sender[]' \
'*-F[]' \
'-C[auto-ignore files in the same way CVS does]' \
'--cvs-exclude[auto-ignore files in the same way CVS does]' \
'--from0[]' \
'-h[Print help]' \
'--help[Print help]' \
':src:_default' \
':dst:_default' \
&& ret=0
}

(( $+functions[_oc-rsync_commands] )) ||
_oc-rsync_commands() {
    local commands; commands=()
    _describe -t commands 'oc-rsync commands' commands "$@"
}

if [ "$funcstack[1]" = "_oc-rsync" ]; then
    _oc-rsync "$@"
else
    compdef _oc-rsync oc-rsync
fi
