# Daemon Mode

`rsync-rs` can act as a standalone daemon that listens on TCP port 873 and serves one or more exported modules. The daemon is started with `--daemon` and at least one `--module` declaration of the form `name=path`.

## Module setup

Modules map a name to a directory on disk. Each module is supplied on the command line:

```bash
rsync-rs --daemon --module data=/srv/export
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

By default `rsync-rs` maps user and group names when transferring ownership metadata. Supplying `--numeric-ids` disables this mapping and preserves raw UID and GID values during synchronization. This flag applies equally in daemon mode and when invoking a client:

```bash
rsync-rs --daemon --numeric-ids --module data=/srv/export
```

## Chroot and privilege drop

Before serving files the daemon confines itself to the module root. On Unix platforms it performs a `chroot` to the module path, changes its working directory to `/`, and drops privileges to the nobody user and group (UID/GID 65534).

## Hosts allow/deny lists

The daemon can restrict connections based on client address. The `--hosts-allow`
and `--hosts-deny` flags accept comma separated IP addresses. A client must match
the allow list (if supplied) and must not match the deny list:

```bash
rsync-rs --daemon \
    --module logs=/srv/logs \
    --hosts-allow=127.0.0.1 \
    --hosts-deny=*
```

Clients whose address does not satisfy these rules are disconnected before any
authentication takes place.

## Logging

Supply `--log-file` to record daemon activity. The optional
`--log-file-format` flag controls the line format and supports `%h` for the
client host and `%m` for the requested module:

```bash
rsync-rs --daemon --module data=/srv/export \
    --log-file=/var/log/rsyncd.log \
    --log-file-format="%h %m"
```

## Message of the day

Use `--motd` to display a message of the day to connecting clients. Each line in
the file is sent with the `@RSYNCD:` prefix during the handshake. Clients can
suppress this output with the `--no-motd` flag:

```bash
rsync-rs --no-motd rsync://host/module dest/
```

