# oc-rsync daemon

`oc-rsyncd` is the rsync daemon provided by `oc-rsync`. It serves files to remote clients over the rsync protocol.

## Synopsis

```sh
oc-rsync --daemon [OPTIONS]
```

## Description

The daemon can be started either directly via `oc-rsync --daemon` or by a service manager. Daemon behaviour and module definitions are controlled by the configuration file `oc-rsyncd.conf(5)`. Clients connect using `oc-rsync(1)` and specify a module name in the `rsync://` URL.

Windows hosts are supported with the same path and metadata semantics as
Unix systems, including normalization of extended `\\?\` paths and
propagation of ACLs and timestamps.

## Options

These flags influence the daemon when run from the command line:

- `--config=FILE` — read an alternate configuration file.
- `--port=PORT` — listen on a different TCP port.
- `--address=ADDR` — bind to a specific network address.
- `--no-detach` — run in the foreground and log to stderr.
- `--log-file=FILE` — append logs to the given file.

## See also

`oc-rsync(1)`, `oc-rsyncd.conf(5)`
