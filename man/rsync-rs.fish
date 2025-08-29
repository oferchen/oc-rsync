complete -c rsync-rs -l compress-choice -l zc -d 'choose the compression algorithm (aka --zc)' -r
complete -c rsync-rs -l compress-level -l zl -d 'explicitly set compression level' -r
complete -c rsync-rs -l partial-dir -d 'put a partially transferred file into DIR' -r -F
complete -c rsync-rs -l bwlimit -d 'throttle I/O bandwidth to RATE bytes per second' -r
complete -c rsync-rs -l link-dest -d 'hardlink to files in DIR when unchanged' -r -F
complete -c rsync-rs -l copy-dest -d 'copy files from DIR when unchanged' -r -F
complete -c rsync-rs -l compare-dest -d 'skip files that match in DIR' -r -F
complete -c rsync-rs -l config -d 'supply a custom configuration file' -r -F
complete -c rsync-rs -l known-hosts -d 'path to SSH known hosts file' -r -F
complete -c rsync-rs -l password-file -d 'read daemon-access password from FILE' -r -F
complete -c rsync-rs -s e -l rsh -d 'specify the remote shell to use' -r
complete -c rsync-rs -l rsync-path -d 'specify the rsync to run on remote machine' -r -F
complete -c rsync-rs -s f -l filter -d 'filter rules provided directly' -r
complete -c rsync-rs -l filter-file -d 'files containing filter rules' -r -F
complete -c rsync-rs -l include -d 'include files matching PATTERN' -r
complete -c rsync-rs -l exclude -d 'exclude files matching PATTERN' -r
complete -c rsync-rs -l include-from -d 'read include patterns from FILE' -r -F
complete -c rsync-rs -l exclude-from -d 'read exclude patterns from FILE' -r -F
complete -c rsync-rs -l files-from -d 'read list of files from FILE' -r -F
complete -c rsync-rs -l local -d 'perform a local sync'
complete -c rsync-rs -s a -l archive -d 'archive mode'
complete -c rsync-rs -s r -l recursive -d 'copy directories recursively'
complete -c rsync-rs -s R -l relative -d 'use relative path names'
complete -c rsync-rs -s n -l dry-run -d 'perform a trial run with no changes made'
complete -c rsync-rs -s S -l sparse -d 'turn sequences of nulls into sparse blocks and preserve existing holes (requires filesystem support)'
complete -c rsync-rs -s v -l verbose -d 'increase logging verbosity'
complete -c rsync-rs -s q -l quiet -d 'suppress non-error messages'
complete -c rsync-rs -l no-motd -d 'suppress daemon-mode MOTD'
complete -c rsync-rs -l delete -d 'remove extraneous files from the destination'
complete -c rsync-rs -l delete-before -d 'receiver deletes before transfer, not during'
complete -c rsync-rs -l delete-during -d 'receiver deletes during the transfer'
complete -c rsync-rs -l delete-after -d 'receiver deletes after transfer, not during'
complete -c rsync-rs -l delete-delay -d 'find deletions during, delete after'
complete -c rsync-rs -l delete-excluded -d 'also delete excluded files from destination'
complete -c rsync-rs -s c -l checksum -d 'use full checksums to determine file changes'
complete -c rsync-rs -l perms -d 'preserve permissions'
complete -c rsync-rs -l times -d 'preserve modification times'
complete -c rsync-rs -s U -l atimes -d 'preserve access times'
complete -c rsync-rs -s N -l crtimes -d 'preserve create times'
complete -c rsync-rs -l owner -d 'preserve owner'
complete -c rsync-rs -l group -d 'preserve group'
complete -c rsync-rs -l links -d 'copy symlinks as symlinks'
complete -c rsync-rs -l hard-links -d 'preserve hard links'
complete -c rsync-rs -l devices -d 'preserve device files'
complete -c rsync-rs -l specials -d 'preserve special files'
complete -c rsync-rs -s z -l compress -d 'compress file data during the transfer (zlib by default, negotiates zstd when supported)'
complete -c rsync-rs -l modern -d 'enable BLAKE3 checksums (zstd is negotiated automatically)'
complete -c rsync-rs -l partial -d 'keep partially transferred files'
complete -c rsync-rs -l progress -d 'show progress during transfer'
complete -c rsync-rs -s P -d 'keep partially transferred files and show progress'
complete -c rsync-rs -l append -d 'append data onto shorter files'
complete -c rsync-rs -l append-verify -d '--append with old data verification'
complete -c rsync-rs -s I -l inplace -d 'update destination files in-place'
complete -c rsync-rs -l numeric-ids -d 'don\'t map uid/gid values by user/group name'
complete -c rsync-rs -l stats -d 'display transfer statistics on completion'
complete -c rsync-rs -l no-host-key-checking -d 'disable strict host key checking (not recommended)'
complete -c rsync-rs -l server -d 'run in server mode (internal use)'
complete -c rsync-rs -l sender -d 'run in sender mode (internal use)'
complete -c rsync-rs -s F -d 'shorthand for per-directory filter files'
complete -c rsync-rs -l from0 -d 'treat file lists as null-separated'
complete -c rsync-rs -s h -l help -d 'Print help (see more with \'--help\')'
