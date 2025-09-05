# Systemd unit hardening

The `oc-rsyncd.service` unit is configured with a minimal capability set. Only
`CAP_NET_BIND_SERVICE` is retained so the daemon can listen on the privileged
rsync port 873 while running as the unprivileged `ocrsync` user.

The service file assumes the binaries are installed in `/usr/bin` (`/usr/bin/oc-rsyncd`
and `/usr/bin/oc-rsync`). If your distribution installs them elsewhere, override
`ExecStart` via a systemd drop-in.

```
$ systemd-analyze security --offline=yes oc-rsyncd.service
→ Overall exposure level for oc-rsyncd.service: 1.3 OK 🙂
```
