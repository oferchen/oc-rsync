json\t''"
complete -c oc-rsync -l log-file -r -F
complete -c oc-rsync -l log-file-format -r
complete -c oc-rsync -l info -r -f -a "backup\t''
copy\t''
del\t''
flist\t''
misc\t''
name\t''
progress\t''
stats\t''"
complete -c oc-rsync -l debug -r -f -a "backup\t''
copy\t''
del\t''
flist\t''
hash\t''
match\t''
misc\t''
options\t''"
complete -c oc-rsync -l max-delete -r
complete -c oc-rsync -l max-alloc -r
complete -c oc-rsync -l max-size -r
complete -c oc-rsync -l min-size -r
complete -c oc-rsync -l backup-dir -r -F
complete -c oc-rsync -l checksum-choice -l cc -r
complete -c oc-rsync -l checksum-seed -d 'set block/file checksum seed (advanced)' -r
complete -c oc-rsync -l chmod -r
complete -c oc-rsync -l chown -r
complete -c oc-rsync -l copy-as -r
complete -c oc-rsync -l usermap -r
complete -c oc-rsync -l groupmap -r
complete -c oc-rsync -l compress-choice -l zc -r
complete -c oc-rsync -l compress-level -l zl -r
complete -c oc-rsync -l skip-compress -r
complete -c oc-rsync -l partial-dir -r -F
complete -c oc-rsync -s T -l temp-dir -r -F
complete -c oc-rsync -l bwlimit -r
complete -c oc-rsync -l timeout -r
complete -c oc-rsync -l contimeout -r
complete -c oc-rsync -l modify-window -r
complete -c oc-rsync -l protocol -d 'force an older protocol version' -r
complete -c oc-rsync -l port -r
complete -c oc-rsync -s B -l block-size -r
complete -c oc-rsync -l link-dest -r -F
complete -c oc-rsync -l copy-dest -r -F
complete -c oc-rsync -l compare-dest -r -F
complete -c oc-rsync -l config -r -F
complete -c oc-rsync -l known-hosts -r -F
complete -c oc-rsync -l password-file -r -F
complete -c oc-rsync -l early-input -r -F
complete -c oc-rsync -s e -l rsh -r
complete -c oc-rsync -s M -l remote-option -d 'send OPTION to the remote side only' -r
complete -c oc-rsync -l sockopts -r
complete -c oc-rsync -l write-batch -r -F
complete -c oc-rsync -l rsync-path -r
complete -c oc-rsync -s f -l filter -r
complete -c oc-rsync -l filter-file -r -F
complete -c oc-rsync -l include -r
complete -c oc-rsync -l exclude -r
complete -c oc-rsync -l include-from -r -F
complete -c oc-rsync -l exclude-from -r -F
complete -c oc-rsync -l files-from -r -F
complete -c oc-rsync -l module -r
complete -c oc-rsync -l address -r
complete -c oc-rsync -l secrets-file -r -F
complete -c oc-rsync -l hosts-allow -r
complete -c oc-rsync -l hosts-deny -r
complete -c oc-rsync -l motd -r -F
complete -c oc-rsync -l lock-file -r -F
complete -c oc-rsync -l state-dir -r -F
complete -c oc-rsync -l peer-version -r
complete -c oc-rsync -l local
complete -c oc-rsync -s a -l archive
complete -c oc-rsync -s r -l recursive
complete -c oc-rsync -s d -l dirs
complete -c oc-rsync -s R -l relative
complete -c oc-rsync -s n -l dry-run
complete -c oc-rsync -l list-only
complete -c oc-rsync -s S -l sparse
complete -c oc-rsync -s u -l update
complete -c oc-rsync -l existing
complete -c oc-rsync -l ignore-existing
complete -c oc-rsync -s m -l prune-empty-dirs
complete -c oc-rsync -l size-only
complete -c oc-rsync -s I -l ignore-times
complete -c oc-rsync -s v -l verbose
complete -c oc-rsync -l human-readable
complete -c oc-rsync -s q -l quiet
complete -c oc-rsync -l no-motd
complete -c oc-rsync -s 8 -l 8-bit-output
complete -c oc-rsync -s i -l itemize-changes -d 'output a change-summary for all updates'
complete -c oc-rsync -l delete
complete -c oc-rsync -l delete-before
complete -c oc-rsync -l delete-during -l del
complete -c oc-rsync -l delete-after
complete -c oc-rsync -l delete-delay
complete -c oc-rsync -l delete-excluded
complete -c oc-rsync -l delete-missing-args
complete -c oc-rsync -l ignore-missing-args
complete -c oc-rsync -l remove-source-files
complete -c oc-rsync -l ignore-errors
complete -c oc-rsync -l preallocate -d 'allocate dest files before writing them'
complete -c oc-rsync -s b -l backup
complete -c oc-rsync -s c -l checksum
complete -c oc-rsync -l perms
complete -c oc-rsync -s E -l executability
complete -c oc-rsync -l times
complete -c oc-rsync -s U -l atimes
complete -c oc-rsync -s N -l crtimes
complete -c oc-rsync -s O -l omit-dir-times
complete -c oc-rsync -s J -l omit-link-times
complete -c oc-rsync -l owner
complete -c oc-rsync -l group
complete -c oc-rsync -l links
complete -c oc-rsync -s L -l copy-links
complete -c oc-rsync -s k -l copy-dirlinks
complete -c oc-rsync -s K -l keep-dirlinks
complete -c oc-rsync -l copy-unsafe-links
complete -c oc-rsync -l safe-links
complete -c oc-rsync -l hard-links
complete -c oc-rsync -l devices
complete -c oc-rsync -l specials
complete -c oc-rsync -l fake-super
complete -c oc-rsync -l super
complete -c oc-rsync -s z -l compress
complete -c oc-rsync -l partial
complete -c oc-rsync -l progress
complete -c oc-rsync -l blocking-io
complete -c oc-rsync -l fsync
complete -c oc-rsync -s y -l fuzzy
complete -c oc-rsync -s P
complete -c oc-rsync -l append
complete -c oc-rsync -l append-verify
complete -c oc-rsync -l inplace
complete -c oc-rsync -l delay-updates
complete -c oc-rsync -s 4 -l ipv4
complete -c oc-rsync -s 6 -l ipv6
complete -c oc-rsync -s W -l whole-file
complete -c oc-rsync -l no-whole-file
complete -c oc-rsync -l numeric-ids
complete -c oc-rsync -l stats
complete -c oc-rsync -l no-host-key-checking
complete -c oc-rsync -s s -l secluded-args -d 'use the protocol to safely send the args'
complete -c oc-rsync -l copy-devices
complete -c oc-rsync -l write-devices -d 'write to devices as files (implies --inplace)'
complete -c oc-rsync -l server
complete -c oc-rsync -l sender
complete -c oc-rsync -s F
complete -c oc-rsync -s C -l cvs-exclude -d 'auto-ignore files in the same way CVS does'
complete -c oc-rsync -l from0
complete -c oc-rsync -l daemon
complete -c oc-rsync -l probe -r
complete -c oc-rsync -s h -l help -d 'Print help'
