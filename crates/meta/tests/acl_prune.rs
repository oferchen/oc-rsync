// crates/meta/tests/acl_prune.rs
#![cfg(feature = "acl")]

#[cfg(feature = "xattr")]
use meta::encode_acl;
use meta::{read_acl, write_acl};
use posix_acl::{ACL_READ, PosixACL, Qualifier};
use std::fs;
use tempfile::tempdir;

fn acl_to_io(err: posix_acl::ACLError) -> std::io::Error {
    if let Some(ioe) = err.as_io_error() {
        if let Some(code) = ioe.raw_os_error() {
            std::io::Error::from_raw_os_error(code)
        } else {
            std::io::Error::new(ioe.kind(), ioe.to_string())
        }
    } else {
        std::io::Error::other(err)
    }
}

#[test]
fn read_prunes_trivial_acls() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("f");
    fs::write(&file, b"hi")?;
    let (acl, dacl) = read_acl(&file, false)?;
    assert!(acl.is_empty());
    assert!(dacl.is_empty());

    let subdir = dir.path().join("d");
    fs::create_dir(&subdir)?;
    let mut dacl = PosixACL::new(0o777);
    dacl.write_default_acl(&subdir).map_err(acl_to_io)?;
    let (_, default_acl) = read_acl(&subdir, false)?;
    assert!(default_acl.is_empty());
    Ok(())
}

#[test]
fn write_prunes_and_removes_default() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("d");
    fs::create_dir(&path)?;

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    let default_entries = dacl.entries();
    write_acl(&path, &[], &default_entries, false, false)?;
    let (_, applied) = read_acl(&path, false)?;
    assert_eq!(applied, default_entries);

    let trivial_acl = PosixACL::new(0o755).entries();
    write_acl(&path, &trivial_acl, &[], false, false)?;
    let (acl_after, _) = read_acl(&path, false)?;
    assert!(acl_after.is_empty());

    let trivial_dacl = PosixACL::new(0o777).entries();
    write_acl(&path, &[], &trivial_dacl, false, false)?;
    let (_, d_after) = read_acl(&path, false)?;
    assert!(d_after.is_empty());
    Ok(())
}

#[cfg(feature = "xattr")]
#[test]
fn fake_super_stores_acls() -> std::io::Result<()> {
    let dir = tempdir()?;
    let file = dir.path().join("f");
    fs::write(&file, b"hi")?;
    let mut acl = PosixACL::read_acl(&file).map_err(acl_to_io)?;
    acl.set(Qualifier::User(12345), ACL_READ);
    let entries = acl.entries();
    write_acl(&file, &entries, &[], true, false)?;
    let stored = xattr::get(&file, "user.rsync.acl")?.unwrap();
    assert_eq!(stored, encode_acl(&entries));

    let subdir = dir.path().join("d");
    fs::create_dir(&subdir)?;
    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::Group(54321), ACL_READ);
    let dentries = dacl.entries();
    write_acl(&subdir, &[], &dentries, true, false)?;
    let stored_d = xattr::get(&subdir, "user.rsync.dacl")?.unwrap();
    assert_eq!(stored_d, encode_acl(&dentries));
    Ok(())
}
