# Transport

The transport layer handles network communication for `oc-rsync`.
It currently supports plain TCP connections and SSH tunnels.

Connection attempts are made once and any errors are reported
immediately without automatic retries.

On Windows, path handling uses extended `\\?\` prefixes and file
metadata such as ACLs and timestamps are preserved to maintain
parity with Unix platforms.
