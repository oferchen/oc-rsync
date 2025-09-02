// crates/meta/tests/acl_roundtrip.rs
#![cfg(feature = "acl")]

use meta::{read_acl, write_acl};
use posix_acl::{PosixACL, Qualifier, ACL_READ};
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
fn roundtrip_acl_rw() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, b"hello")?;
    fs::write(&dst, b"world")?;

    let mut acl = PosixACL::read_acl(&src).map_err(acl_to_io)?;
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&src).map_err(acl_to_io)?;

    let (acl_entries, _) = read_acl(&src, false)?;
    write_acl(&dst, &acl_entries, &[], false, false)?;
    let (applied, _) = read_acl(&dst, false)?;
    assert_eq!(acl_entries, applied);
    Ok(())
}

#[test]
fn roundtrip_default_acl_rw() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir(&src)?;
    fs::create_dir(&dst)?;

    let mut acl = PosixACL::read_acl(&src).map_err(acl_to_io)?;
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&src).map_err(acl_to_io)?;

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::Group(54321), ACL_READ);
    dacl.write_default_acl(&src).map_err(acl_to_io)?;

    let (acl_entries, default_entries) = read_acl(&src, false)?;
    write_acl(&dst, &acl_entries, &default_entries, false, false)?;
    let (acl_applied, dacl_applied) = read_acl(&dst, false)?;
    assert_eq!(acl_entries, acl_applied);
    assert_eq!(default_entries, dacl_applied);
    Ok(())
}
