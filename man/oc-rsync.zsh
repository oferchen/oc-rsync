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
'--log-file=[]:FILE:_files' \
'--log-file-format=[]:FMT:_default' \
'*--info=[]:FLAGS:(backup copy del flist misc name progress stats)' \
'*--debug=[]:FLAGS:(backup copy del flist hash match misc options)' \
'--max-delete=[]:NUM:_default' \
'--max-alloc=[]:SIZE:_default' \
'--max-size=[]:SIZE:_default' \
'--min-size=[]:SIZE:_default' \
'--backup-dir=[]:DIR:_files' \
'--checksum-choice=[]:STR:_default' \
'--cc=[]:STR:_default' \
'--checksum-seed=[set block/file checksum seed (advanced)]:NUM:_default' \
'*--chmod=[]:CHMOD:_default' \
'--chown=[]:USER:GROUP:_default' \
'--copy-as=[]:USER[:GROUP]:_default' \
'*--usermap=[]:FROM:TO:_default' \
'*--groupmap=[]:FROM:TO:_default' \
'--compress-choice=[]:STR:_default' \
'--zc=[]:STR:_default' \
'--compress-level=[]:NUM:_default' \
'--zl=[]:NUM:_default' \
'*--skip-compress=[]:LIST:_default' \
'--partial-dir=[]:DIR:_files' \
'-T+[]:DIR:_files' \
'--temp-dir=[]:DIR:_files' \
'--bwlimit=[]:RATE:_default' \
'--timeout=[]:SECONDS:_default' \
'--contimeout=[]:SECONDS:_default' \
'--modify-window=[]:SECONDS:_default' \
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
'*-M+[send OPTION to the remote side only]:OPT:_default' \
'*--remote-option=[send OPTION to the remote side only]:OPT:_default' \
'*--sockopts=[]:OPTIONS:_default' \
'--write-batch=[]:FILE:_files' \
'--rsync-path=[]:PATH:_default' \
'*-f+[]:RULE:_default' \
'*--filter=[]:RULE:_default' \
'*--filter-file=[]:FILE:_files' \
'*--include=[]:PATTERN:_default' \
'*--exclude=[]:PATTERN:_default' \
'*--include-from=[]:FILE:_files' \
'*--exclude-from=[]:FILE:_files' \
'*--files-from=[]:FILE:_files' \
'*--module=[]:NAME=PATH:_default' \
'--address=[]:ADDRESS:_default' \
'--secrets-file=[]:FILE:_files' \
'*--hosts-allow=[]:LIST:_default' \
'*--hosts-deny=[]:LIST:_default' \
'--motd=[]:FILE:_files' \
'--lock-file=[]:FILE:_files' \
'--state-dir=[]:DIR:_files' \
'--peer-version=[]:VER:_default' \
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
'--existing[]' \
'--ignore-existing[]' \
'-m[]' \
'--prune-empty-dirs[]' \
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
'--del[]' \
'--delete-after[]' \
'--delete-delay[]' \
'--delete-excluded[]' \
'--delete-missing-args[]' \
'--ignore-missing-args[]' \
'--remove-source-files[]' \
'--ignore-errors[]' \
'--preallocate[allocate dest files before writing them]' \
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
'-K[]' \
'--keep-dirlinks[]' \
'--copy-unsafe-links[]' \
'--safe-links[]' \
'--hard-links[]' \
'--devices[]' \
'--specials[]' \
'--fake-super[]' \
'--super[]' \
'-z[]' \
'--compress[]' \
'--partial[]' \
'--progress[]' \
'--blocking-io[]' \
'--fsync[]' \
'-y[]' \
'--fuzzy[]' \
'-P[]' \
'--append[]' \
'--append-verify[]' \
'--inplace[]' \
'--delay-updates[]' \
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
'-s[use the protocol to safely send the args]' \
'--secluded-args[use the protocol to safely send the args]' \
'--copy-devices[]' \
'--write-devices[write to devices as files (implies --inplace)]' \
'--server[]' \
'--sender[]' \
'*-F[]' \
'-C[auto-ignore files in the same way CVS does]' \
'--cvs-exclude[auto-ignore files in the same way CVS does]' \
'--from0[]' \
'--daemon[]' \
'--probe[]::ADDR:' \
'-h[Print help]' \
'--help[Print help]' \
'::src:_default' \
'::dst:_default' \
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
