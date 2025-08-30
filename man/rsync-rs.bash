_rsync-rs() {
    local i cur prev opts cmd
    COMPREPLY=()
    if [[ "${BASH_VERSINFO[0]}" -ge 4 ]]; then
        cur="$2"
    else
        cur="${COMP_WORDS[COMP_CWORD]}"
    fi
    prev="$3"
    cmd=""
    opts=""

    for i in "${COMP_WORDS[@]:0:COMP_CWORD}"
    do
        case "${cmd},${i}" in
            ",$1")
                cmd="rsync__rs"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        rsync__rs)
            opts="-a -r -d -R -n -S -u -I -v -q -i -b -c -E -U -N -O -J -L -k -z -T -P -4 -6 -B -W -e -f -F -C -h --local --archive --recursive --dirs --relative --dry-run --list-only --sparse --update --ignore-existing --size-only --ignore-times --verbose --human-readable --quiet --no-motd --itemize-changes --delete --delete-before --delete-during --delete-after --delete-delay --delete-excluded --backup --backup-dir --checksum --cc --checksum-choice --checksum-seed --perms --executability --chmod --chown --times --atimes --crtimes --omit-dir-times --omit-link-times --owner --group --links --copy-links --copy-dirlinks --copy-unsafe-links --safe-links --hard-links --devices --specials --compress --zc --compress-choice --zl --compress-level --skip-compress --modern --partial --partial-dir --temp-dir --progress --append --append-verify --inplace --bwlimit --timeout --contimeout --port --ipv4 --ipv6 --block-size --whole-file --no-whole-file --link-dest --copy-dest --compare-dest --numeric-ids --stats --config --known-hosts --no-host-key-checking --password-file --rsh --server --sender --rsync-path --filter --filter-file --cvs-exclude --include --exclude --include-from --exclude-from --files-from --from0 --help <SRC> <DST>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --backup-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --checksum-choice)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --cc)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --checksum-seed)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --chmod)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --chown)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --compress-choice)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --zc)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --compress-level)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --zl)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --skip-compress)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --partial-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --temp-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -T)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --bwlimit)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --timeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --contimeout)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --port)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --block-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -B)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --link-dest)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --copy-dest)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --compare-dest)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --config)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --known-hosts)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --password-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --rsh)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -e)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --rsync-path)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -f)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --filter-file)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --include)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --exclude)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --include-from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --exclude-from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --files-from)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                *)
                    COMPREPLY=()
                    ;;
            esac
            COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
            return 0
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _rsync-rs -o nosort -o bashdefault -o default rsync-rs
else
    complete -F _rsync-rs -o bashdefault -o default rsync-rs
fi
