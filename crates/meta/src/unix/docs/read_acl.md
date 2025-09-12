Convenience wrapper around [`read_acl_inner`] that infers metadata from `path`.

* `path` - File or directory whose ACLs should be read.
* `fake_super` - Read ACLs from xattrs instead of the filesystem when `true`.

