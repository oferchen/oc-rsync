// crates/engine/tests/acls.rs
#![doc = "ACL tests skip when unsupported."]
#![cfg(unix)]
#![allow(
    clippy::needless_return,
    clippy::single_match,
    clippy::collapsible_if,
    clippy::redundant_pattern_matching,
    clippy::needless_borrows_for_generic_args
)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use posix_acl::{ACL_READ, ACL_WRITE, PosixACL, Qualifier};
use tempfile::tempdir;

mod tests;
#[cfg(feature = "acl")]
#[test]
fn acls_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();

    let mut acl = match PosixACL::read_acl(&file) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    acl.set(Qualifier::User(12345), ACL_READ);
    if let Err(_) = acl.write_acl(&file) {
        eprintln!("Skipping acls_roundtrip test: ACLs not supported");
        return;
    }

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    if let Err(_) = dacl.write_default_acl(&src) {
        eprintln!("Skipping acls_roundtrip test: ACLs not supported");
        return;
    }

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            ..Default::default()
        },
    )
    .unwrap();

    let acl_src = match PosixACL::read_acl(&file) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    let acl_dst = match PosixACL::read_acl(&dst.join("file")) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let dacl_src = match PosixACL::read_default_acl(&src) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    let dacl_dst = match PosixACL::read_default_acl(&dst) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
}

#[cfg(feature = "acl")]
#[test]
fn root_acl_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let mut acl = PosixACL::new(0o755);
    acl.set(Qualifier::User(12345), ACL_READ);
    if let Err(_) = acl.write_acl(&src) {
        eprintln!("Skipping root_acl_roundtrip test: ACLs not supported");
        return;
    }

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            ..Default::default()
        },
    )
    .unwrap();

    let acl_src = match PosixACL::read_acl(&src) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping root_acl_roundtrip test: ACLs not supported");
            return;
        }
    };
    let acl_dst = match PosixACL::read_acl(&dst) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping root_acl_roundtrip test: ACLs not supported");
            return;
        }
    };
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[cfg(feature = "acl")]
#[test]
fn acls_imply_perms() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&file).unwrap();

    let dst_file = dst.join("file");
    fs::write(&dst_file, b"junk").unwrap();
    fs::set_permissions(&dst_file, fs::Permissions::from_mode(0o600)).unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            perms: false,
            ..Default::default()
        },
    )
    .unwrap();

    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}
#[cfg(feature = "acl")]
#[test]
fn acls_roundtrip_default_acl() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ | ACL_WRITE);
    acl.write_acl(&file).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            ..Default::default()
        },
    )
    .unwrap();
    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}
