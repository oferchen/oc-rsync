Write access and default ACLs to a filesystem object and optionally store
them as fake-super xattrs.

* `path` - Target file or directory.
* `acl` - Access ACL entries to apply.
* `default_acl` - Optional default ACL entries for directories.
* `fake_super` - When `true`, store ACLs as xattrs instead of applying them.
* `super_user` - Indicates whether the process has super-user privileges.
