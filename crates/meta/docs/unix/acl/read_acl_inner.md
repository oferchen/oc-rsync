Read access and default ACLs for a filesystem object.

* `path` - File or directory to inspect.
* `is_dir` - Indicates whether `path` is a directory.
* `fake_super` - Read ACLs from xattrs instead of the filesystem when `true`.
* `mode` - Mode bits used to detect trivial ACLs.
