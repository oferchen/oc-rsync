# Systemd unit hardening

The `oc-rsyncd.service` unit is configured with a minimal capability set. Only
`CAP_NET_BIND_SERVICE` is retained so the daemon can listen on the privileged
rsync port 873 while running as the unprivileged `ocrsync` user.

```
$ systemd-analyze security --offline=yes oc-rsyncd.service
→ Overall exposure level for oc-rsyncd.service: 1.3 OK 🙂
```
