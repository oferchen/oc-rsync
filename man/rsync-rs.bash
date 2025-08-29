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
            opts="-a -r -R -n -S -v -q -c -U -N -z -P -I -e -f -F -h --local --archive --recursive --relative --dry-run --sparse --verbose --quiet --no-motd --delete --delete-before --delete-during --delete-after --delete-delay --delete-excluded --checksum --perms --times --atimes --crtimes --owner --group --links --hard-links --devices --specials --compress --zc --compress-choice --zl --compress-level --modern --partial --partial-dir --progress --append --append-verify --inplace --bwlimit --link-dest --copy-dest --compare-dest --numeric-ids --stats --config --known-hosts --no-host-key-checking --password-file --rsh --server --sender --rsync-path --filter --filter-file --include --exclude --include-from --exclude-from --files-from --from0 --help <SRC> <DST>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
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
                --partial-dir)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --bwlimit)
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
