# Systemd unit hardening

The `oc-rsyncd.service` unit is configured with a minimal capability set.
`CAP_DAC_READ_SEARCH`, `CAP_FOWNER`, `CAP_CHOWN`, and `CAP_DAC_OVERRIDE` are
retained so the daemon can adjust ownership and bypass permission checks while
running as the unprivileged `ocrsync` user.

The service file assumes the binaries are installed in `/usr/bin` (`/usr/bin/oc-rsync`
and `/usr/bin/oc-rsyncd`). If your distribution installs them elsewhere, override
`ExecStart` via a systemd drop-in.

```
$ systemd-analyze security --offline=yes oc-rsyncd.service
→ Overall exposure level for oc-rsyncd.service: 1.3 OK 🙂
```
