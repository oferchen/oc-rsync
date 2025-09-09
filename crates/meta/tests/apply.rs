// crates/meta/tests/apply.rs
use filetime::FileTime;
use meta::{Metadata, Options};
use std::fs;
use tempfile::tempdir;

#[cfg(all(unix, feature = "xattr"))]
use nix::unistd::{getgid, getuid};
#[cfg(all(unix, feature = "xattr"))]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn apply_permissions_and_ownership() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("file");
    fs::write(&path, b"data")?;
    let uid = getuid().as_raw();
    let gid = getgid().as_raw();
    let meta = Metadata {
        uid,
        gid,
        mode: 0o640,
        mtime: FileTime::from_unix_time(0, 0),
        atime: None,
        crtime: None,
        xattrs: Vec::new(),
        #[cfg(feature = "acl")]
        acl: Vec::new(),
        #[cfg(feature = "acl")]
        default_acl: Vec::new(),
    };
    meta.apply(
        &path,
        Options {
            owner: true,
            group: true,
            perms: true,
            ..Default::default()
        },
    )?;
    let m = fs::metadata(&path)?;
    assert_eq!(m.permissions().mode() & 0o777, 0o640);
    assert_eq!(m.uid(), uid);
    assert_eq!(m.gid(), gid);
    Ok(())
}

#[cfg(windows)]
#[test]
fn apply_permissions_and_ownership() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("file");
    fs::write(&path, b"data")?;
    let meta = Metadata {
        uid: 0,
        gid: 0,
        mode: 0o444,
        mtime: FileTime::from_unix_time(0, 0),
        atime: None,
        crtime: None,
        #[cfg(feature = "acl")]
        acl: Vec::new(),
        #[cfg(feature = "acl")]
        default_acl: Vec::new(),
    };
    meta.apply(
        &path,
        Options {
            owner: true,
            group: true,
            perms: true,
            ..Default::default()
        },
    )?;
    let m = fs::metadata(&path)?;
    assert!(m.permissions().readonly());
    Ok(())
}
