Return `true` if manipulating POSIX ACLs is supported on this system.

The check attempts to write an access ACL to a temporary file and a default
ACL to a temporary directory. If both operations succeed, ACLs are
considered supported.

