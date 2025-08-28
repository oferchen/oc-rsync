use std::fs;

use filetime::FileTime;
use meta::Metadata;
use nix::unistd::{chown, Gid, Uid};
use tempfile::tempdir;

#[test]
fn roundtrip_basic_metadata() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");

    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    // Customize source metadata
    let mode = 0o741;
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    nix::sys::stat::fchmodat(
        None,
        &src,
        nix::sys::stat::Mode::from_bits_truncate(mode),
        nix::sys::stat::FchmodatFlags::NoFollowSymlink,
    )?;
    filetime::set_file_mtime(&src, mtime)?;

    // Make destination different
    nix::sys::stat::fchmodat(
        None,
        &dst,
        nix::sys::stat::Mode::from_bits_truncate(0o600),
        nix::sys::stat::FchmodatFlags::NoFollowSymlink,
    )?;
    filetime::set_file_mtime(&dst, FileTime::from_unix_time(1, 0))?;
    chown(&dst, Some(Uid::from_raw(1)), Some(Gid::from_raw(1)))?;

    let meta = Metadata::from_path(&src)?;
    meta.apply(&dst)?;
    let applied = Metadata::from_path(&dst)?;

    assert_eq!(meta.uid, applied.uid);
    assert_eq!(meta.gid, applied.gid);
    assert_eq!(meta.mode, applied.mode);
    assert_eq!(meta.mtime, applied.mtime);
    Ok(())
}

#[cfg(feature = "xattr")]
#[test]
fn roundtrip_xattrs() -> std::io::Result<()> {
    use xattr;

    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    xattr::set(&src, "user.test", b"value")?;

    let meta = Metadata::from_path(&src)?;
    meta.apply(&dst)?;
    let applied = Metadata::from_path(&dst)?;
    let filter = |xs: &[(std::ffi::OsString, Vec<u8>)]| {
        xs.iter()
            .filter(|(n, _)| n != "system.posix_acl_access")
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(filter(&meta.xattrs), filter(&applied.xattrs));
    Ok(())
}

#[cfg(feature = "acl")]
#[test]
fn roundtrip_acl() -> std::io::Result<()> {
    use posix_acl::{PosixACL, Qualifier, ACL_READ};

    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    // Add extra ACL entry to source
    let mut acl = PosixACL::read_acl(&src).map_err(|e| {
        if let Some(ioe) = e.as_io_error() {
            if let Some(code) = ioe.raw_os_error() {
                std::io::Error::from_raw_os_error(code)
            } else {
                std::io::Error::new(ioe.kind(), ioe.to_string())
            }
        } else {
            std::io::Error::new(std::io::ErrorKind::Other, e)
        }
    })?;
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&src).map_err(|e| {
        if let Some(ioe) = e.as_io_error() {
            if let Some(code) = ioe.raw_os_error() {
                std::io::Error::from_raw_os_error(code)
            } else {
                std::io::Error::new(ioe.kind(), ioe.to_string())
            }
        } else {
            std::io::Error::new(std::io::ErrorKind::Other, e)
        }
    })?;

    let meta = Metadata::from_path(&src)?;
    meta.apply(&dst)?;
    let applied = Metadata::from_path(&dst)?;

    assert_eq!(meta.acl, applied.acl);
    Ok(())
}
