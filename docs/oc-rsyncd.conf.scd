oc-rsyncd.conf(5)

# NAME

oc-rsyncd.conf - configuration file for the oc-rsync daemon

# DESCRIPTION

The file *oc-rsyncd.conf* controls the behaviour of the `oc-rsync` daemon.
It uses an INI-like syntax modelled after `rsyncd.conf`.

Global parameters appear at the top of the file.  Module sections
enclosed in brackets export directories to remote clients.

A minimal configuration exporting */srv/data*:

\[data\]
path = /srv/data

Key names are case insensitive.  Dashes, underscores and spaces are treated
interchangeably.

# GLOBAL OPTIONS

## port = NUMBER

Listen on the given TCP port instead of 873.

## address = ADDRESS

Bind to the supplied interface.

## secrets-file = PATH

Authenticate clients using tokens from this file.

# MODULE OPTIONS

## path = PATH

Directory to export. Required.

## read only = BOOL

Disallow modifications when set to "true".

## hosts allow = IP[,IP...]

Permit only the listed client addresses.

## hosts deny = IP[,IP...]

Reject connections from the listed addresses.

# FILES

/etc/oc-rsyncd.conf - default configuration path

# SEE ALSO

oc-rsync(1), oc-rsyncd(8)

