# Daemon Mode

`oc-rsync` can act as a standalone daemon that listens on TCP port 873 and serves one or more exported modules. The daemon is started with `--daemon` and at least one `--module` declaration of the form `name=path`.

The default listener binds to all IPv4 interfaces on port 873. Supply
`--port` to choose a different port. The `-4` and `-6` flags restrict the
listener to IPv4 or IPv6 addresses respectively. These can be combined with
`--address` to bind a specific interface.

## Module setup

Modules map a name to a directory on disk. Each module is supplied on the command line:

```bash
oc-rsync --daemon --module 'data=/srv/export'
```

The integration tests spawn a daemon in exactly this manner when negotiating protocol versions.

## Secrets-file authentication

If the daemon finds an `auth` file in its working directory, clients must supply a matching token. The secrets file path can be overridden with `--secrets-file`. The file must be readable only by the daemon user (mode `0600` on Unix) and may list optional modules a token is permitted to access:

```
$ cat auth
s3cr3t data backups
```

During the handshake the client sends the token followed by a newline. The test suite demonstrates that an invalid token is rejected with an `@ERROR` message. Tokens without an explicit module list allow access to any module.

## Numeric ID handling

By default `oc-rsync` maps user and group names when transferring ownership metadata. Supplying `--numeric-ids` disables this mapping and preserves raw UID and GID values during synchronization. This flag applies equally in daemon mode and when invoking a client:

```bash
oc-rsync --daemon --numeric-ids --module 'data=/srv/export'
```

### Ownership and permissions

Setting file ownership or groups requires elevated privileges. The daemon must
run as `root` or possess the `CAP_CHOWN` capability in order to honor the
`--owner`, `--group`, or `--chown` flags from clients. Without these
capabilities the daemon will silently retain its current UID and GID when
creating files, and ownership requests from clients will be ignored.

## Chroot and privilege drop

Before serving files the daemon confines itself to the module root. On Unix platforms it performs a `chroot` to the module path, changes its working directory to `/`, and drops privileges to the nobody user and group (UID/GID 65534).

## Hosts allow/deny lists

The daemon can restrict connections based on client address. The `--hosts-allow`
and `--hosts-deny` flags accept comma separated IP addresses. A client must match
the allow list (if supplied) and must not match the deny list:

```bash
oc-rsync --daemon \
    --module 'logs=/srv/logs' \
    --hosts-allow=127.0.0.1 \
    --hosts-deny=*
```

Clients whose address does not satisfy these rules are disconnected before any
authentication takes place.

Per-module allow and deny lists may also be specified in a configuration file:

```
[data]
path = /srv/data
hosts allow = 192.0.2.10
hosts deny = 192.0.2.20
```

These rules are evaluated after the global lists. Module entries allow fine
grained control when different exports require distinct access policies.

## Logging

Supply `--log-file` to record daemon activity. The optional
`--log-file-format` flag controls the line format and supports `%h` for the
client host and `%m` for the requested module:

```bash
oc-rsync --daemon --module 'data=/srv/export' \
    --log-file=/var/log/rsyncd.log \
    --log-file-format="%h %m"
```

## Message of the day

Use `--motd` to display a message of the day to connecting clients. Each line in
the file is sent with the `@RSYNCD:` prefix during the handshake. Clients can
suppress this output with the `--no-motd` flag:

```bash
oc-rsync --no-motd 'rsync://host/module' 'dest/'
```

