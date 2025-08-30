complete -c rsync-rs -l backup-dir -r -F
complete -c rsync-rs -l checksum-choice -l cc -r
complete -c rsync-rs -l checksum-seed -d 'set block/file checksum seed (advanced)' -r
complete -c rsync-rs -l chmod -r
complete -c rsync-rs -l chown -r
complete -c rsync-rs -l compress-choice -l zc -r
complete -c rsync-rs -l compress-level -l zl -r
complete -c rsync-rs -l skip-compress -r
complete -c rsync-rs -l partial-dir -r -F
complete -c rsync-rs -s T -l temp-dir -r -F
complete -c rsync-rs -l bwlimit -r
complete -c rsync-rs -l timeout -r
complete -c rsync-rs -l contimeout -r
complete -c rsync-rs -l port -r
complete -c rsync-rs -s B -l block-size -r
complete -c rsync-rs -l link-dest -r -F
complete -c rsync-rs -l copy-dest -r -F
complete -c rsync-rs -l compare-dest -r -F
complete -c rsync-rs -l config -r -F
complete -c rsync-rs -l known-hosts -r -F
complete -c rsync-rs -l password-file -r -F
complete -c rsync-rs -s e -l rsh -r
complete -c rsync-rs -l rsync-path -r
complete -c rsync-rs -s f -l filter -r
complete -c rsync-rs -l filter-file -r -F
complete -c rsync-rs -l include -r
complete -c rsync-rs -l exclude -r
complete -c rsync-rs -l include-from -r -F
complete -c rsync-rs -l exclude-from -r -F
complete -c rsync-rs -l files-from -r -F
complete -c rsync-rs -l local
complete -c rsync-rs -s a -l archive
complete -c rsync-rs -s r -l recursive
complete -c rsync-rs -s d -l dirs
complete -c rsync-rs -s R -l relative
complete -c rsync-rs -s n -l dry-run
complete -c rsync-rs -l list-only
complete -c rsync-rs -s S -l sparse
complete -c rsync-rs -s u -l update
complete -c rsync-rs -l ignore-existing
complete -c rsync-rs -l size-only
complete -c rsync-rs -s I -l ignore-times
complete -c rsync-rs -s v -l verbose
complete -c rsync-rs -l human-readable
complete -c rsync-rs -s q -l quiet
complete -c rsync-rs -l no-motd
complete -c rsync-rs -s i -l itemize-changes -d 'output a change-summary for all updates'
complete -c rsync-rs -l delete
complete -c rsync-rs -l delete-before
complete -c rsync-rs -l delete-during
complete -c rsync-rs -l delete-after
complete -c rsync-rs -l delete-delay
complete -c rsync-rs -l delete-excluded
complete -c rsync-rs -s b -l backup
complete -c rsync-rs -s c -l checksum
complete -c rsync-rs -l perms
complete -c rsync-rs -s E -l executability
complete -c rsync-rs -l times
complete -c rsync-rs -s U -l atimes
complete -c rsync-rs -s N -l crtimes
complete -c rsync-rs -s O -l omit-dir-times
complete -c rsync-rs -s J -l omit-link-times
complete -c rsync-rs -l owner
complete -c rsync-rs -l group
complete -c rsync-rs -l links
complete -c rsync-rs -s L -l copy-links
complete -c rsync-rs -s k -l copy-dirlinks
complete -c rsync-rs -l copy-unsafe-links
complete -c rsync-rs -l safe-links
complete -c rsync-rs -l hard-links
complete -c rsync-rs -l devices
complete -c rsync-rs -l specials
complete -c rsync-rs -s z -l compress
complete -c rsync-rs -l modern -d 'Enable zstd compression and BLAKE3 checksums (requires `blake3` feature)'
complete -c rsync-rs -l partial
complete -c rsync-rs -l progress
complete -c rsync-rs -s P
complete -c rsync-rs -l append
complete -c rsync-rs -l append-verify
complete -c rsync-rs -l inplace
complete -c rsync-rs -s 4 -l ipv4
complete -c rsync-rs -s 6 -l ipv6
complete -c rsync-rs -s W -l whole-file
complete -c rsync-rs -l no-whole-file
complete -c rsync-rs -l numeric-ids
complete -c rsync-rs -l stats
complete -c rsync-rs -l no-host-key-checking
complete -c rsync-rs -l server
complete -c rsync-rs -l sender
complete -c rsync-rs -s F
complete -c rsync-rs -s C -l cvs-exclude -d 'auto-ignore files in the same way CVS does'
complete -c rsync-rs -l from0
complete -c rsync-rs -s h -l help -d 'Print help'
