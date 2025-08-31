_oc-rsync() {
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
                cmd="oc__rsync"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        oc__rsync)
            opts="-a -r -d -R -n -S -u -m -I -v -q -8 -i -b -c -E -U -N -O -J -L -k -K -z -T -P -4 -6 -B -W -e -M -s -f -F -C -h --local --archive --recursive --dirs --relative --dry-run --list-only --sparse --update --existing --ignore-existing --prune-empty-dirs --size-only --ignore-times --verbose --log-format --human-readable --quiet --no-motd --8-bit-output --itemize-changes --delete --delete-before --delete-during --delete-after --delete-delay --delete-excluded --delete-missing-args --remove-source-files --ignore-errors --max-delete --max-alloc --max-size --min-size --preallocate --backup --backup-dir --checksum --cc --checksum-choice --checksum-seed --perms --executability --chmod --chown --usermap --groupmap --times --atimes --crtimes --omit-dir-times --omit-link-times --owner --group --links --copy-links --copy-dirlinks --keep-dirlinks --copy-unsafe-links --safe-links --hard-links --devices --specials --compress --zc --compress-choice --zl --compress-level --skip-compress --modern --modern-compress --modern-hash --modern-cdc --modern-cdc-min --modern-cdc-max --partial --partial-dir --temp-dir --progress --blocking-io --append --append-verify --inplace --bwlimit --timeout --contimeout --protocol --port --ipv4 --ipv6 --block-size --whole-file --no-whole-file --link-dest --copy-dest --compare-dest --numeric-ids --stats --config --known-hosts --no-host-key-checking --password-file --early-input --rsh --remote-option --secluded-args --sockopts --write-batch --copy-devices --write-devices --server --sender --rsync-path --filter --filter-file --cvs-exclude --include --exclude --include-from --exclude-from --files-from --from0 --help <SRC> <DST>"
            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi
            case "${prev}" in
                --log-format)
                    COMPREPLY=($(compgen -W "text json" -- "${cur}"))
                    return 0
                    ;;
                --max-delete)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-alloc)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --max-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --min-size)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
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
                --usermap)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --groupmap)
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
                --modern-compress)
                    COMPREPLY=($(compgen -W "auto zstd lz4" -- "${cur}"))
                    return 0
                    ;;
                --modern-hash)
                    COMPREPLY=($(compgen -W "" -- "${cur}"))
                    return 0
                    ;;
                --modern-cdc)
                    COMPREPLY=($(compgen -W "fastcdc off" -- "${cur}"))
                    return 0
                    ;;
                --modern-cdc-min)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --modern-cdc-max)
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
                --protocol)
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
                --early-input)
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
                --remote-option)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                -M)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --sockopts)
                    COMPREPLY=($(compgen -f "${cur}"))
                    return 0
                    ;;
                --write-batch)
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
    complete -F _oc-rsync -o nosort -o bashdefault -o default oc-rsync
else
    complete -F _oc-rsync -o bashdefault -o default oc-rsync
fi
