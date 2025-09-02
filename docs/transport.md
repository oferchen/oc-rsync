# Transport

The transport layer handles network communication for `oc-rsync`.
It currently supports plain TCP connections and SSH tunnels.

Connection attempts are made once and any errors are reported
immediately without automatic retries.
