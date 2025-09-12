Store ACLs in fake-super extended attributes when required.

* `path` - File or directory to annotate.
* `is_dir` - Indicates whether `path` is a directory.
* `fake_super` - Enables fake-super behavior.
* `super_user` - If `false`, ACLs are stored as xattrs.
* `acl` - Access ACL entries.
* `dacl` - Default ACL entries for directories.

