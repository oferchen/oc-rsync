// crates/meta/tests/roundtrip.rs
use std::fs;
use std::os::unix::fs::PermissionsExt;

use filetime::FileTime;
use meta::{Metadata, Options};
use nix::unistd::{chown, Gid, Uid};
use std::time::SystemTime;
use tempfile::tempdir;

#[test]
fn roundtrip_full_metadata() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");

    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    let mode = 0o741;
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    let atime = FileTime::from_system_time(SystemTime::now());
    nix::sys::stat::fchmodat(
        None,
        &src,
        nix::sys::stat::Mode::from_bits_truncate(mode),
        nix::sys::stat::FchmodatFlags::NoFollowSymlink,
    )?;
    filetime::set_file_times(&src, atime, mtime)?;

    nix::sys::stat::fchmodat(
        None,
        &dst,
        nix::sys::stat::Mode::from_bits_truncate(0o600),
        nix::sys::stat::FchmodatFlags::NoFollowSymlink,
    )?;
    filetime::set_file_times(
        &dst,
        FileTime::from_unix_time(1, 0),
        FileTime::from_unix_time(1, 0),
    )?;
    chown(&dst, Some(Uid::from_raw(1)), Some(Gid::from_raw(1)))?;

    let opts = Options {
        owner: true,
        group: true,
        perms: true,
        times: true,
        atimes: true,
        crtimes: true,
        ..Default::default()
    };
    let meta = Metadata::from_path(&src, opts.clone())?;
    meta.apply(&dst, opts.clone())?;
    let applied = Metadata::from_path(&dst, opts)?;

    assert_eq!(meta.uid, applied.uid);
    assert_eq!(meta.gid, applied.gid);
    assert_eq!(meta.mode, applied.mode);
    assert_eq!(meta.mtime, applied.mtime);
    assert_eq!(meta.atime, applied.atime);
    if meta.crtime.is_some() || applied.crtime.is_some() {
        assert_eq!(meta.crtime, applied.crtime);
    }
    Ok(())
}

#[test]
fn default_skips_owner_group_perms() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");

    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    nix::sys::stat::fchmodat(
        None,
        &src,
        nix::sys::stat::Mode::from_bits_truncate(0o741),
        nix::sys::stat::FchmodatFlags::NoFollowSymlink,
    )?;
    nix::sys::stat::fchmodat(
        None,
        &dst,
        nix::sys::stat::Mode::from_bits_truncate(0o600),
        nix::sys::stat::FchmodatFlags::NoFollowSymlink,
    )?;
    chown(&dst, Some(Uid::from_raw(1)), Some(Gid::from_raw(1)))?;

    let orig = Metadata::from_path(&dst, Options::default())?;
    let meta = Metadata::from_path(&src, Options::default())?;
    meta.apply(&dst, Options::default())?;
    let applied = Metadata::from_path(&dst, Options::default())?;

    assert_eq!(orig.uid, applied.uid);
    assert_eq!(orig.gid, applied.gid);
    assert_eq!(orig.mode, applied.mode);
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

    let opts = Options {
        xattrs: true,
        ..Default::default()
    };
    let meta = Metadata::from_path(&src, opts.clone())?;
    meta.apply(&dst, opts.clone())?;
    let applied = Metadata::from_path(&dst, opts)?;
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

    let mut acl = PosixACL::read_acl(&src).map_err(|e| {
        if let Some(ioe) = e.as_io_error() {
            if let Some(code) = ioe.raw_os_error() {
                std::io::Error::from_raw_os_error(code)
            } else {
                std::io::Error::new(ioe.kind(), ioe.to_string())
            }
        } else {
            std::io::Error::other(e)
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
            std::io::Error::other(e)
        }
    })?;

    let opts = Options {
        acl: true,
        ..Default::default()
    };
    let meta = Metadata::from_path(&src, opts.clone())?;
    meta.apply(&dst, opts.clone())?;
    let applied = Metadata::from_path(&dst, opts)?;

    assert_eq!(meta.acl, applied.acl);
    Ok(())
}

#[test]
fn apply_chmod_rule() -> std::io::Result<()> {
    use meta::{Chmod, ChmodOp, ChmodTarget};
    let dir = tempdir()?;
    let file = dir.path().join("file");
    fs::write(&file, b"data")?;
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640))?;

    let opts = Options {
        chmod: Some(vec![Chmod {
            target: ChmodTarget::All,
            op: ChmodOp::Add,
            mask: 0o111,
            bits: 0o111,
            conditional: false,
        }]),
        ..Default::default()
    };
    let meta = Metadata::from_path(&file, opts.clone())?;
    meta.apply(&file, opts)?;
    let applied = fs::metadata(&file)?;
    assert_eq!(applied.permissions().mode() & 0o777, 0o751);
    Ok(())
}
