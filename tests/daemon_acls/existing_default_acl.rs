#![cfg(all(unix, feature = "acl"))]

use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

use meta::{acls_supported, xattrs_supported};
use posix_acl::{ACL_READ, PosixACL, Qualifier};

#[path = "../common/mod.rs"]
mod common;
use common::daemon::{spawn_daemon, wait_for_daemon};

macro_rules! read_acl_or_skip {
    ($path:expr) => {
        match PosixACL::read_acl($path) {
            Ok(acl) => acl,
            Err(err) => {
                eprintln!("skipping: {}", err);
                return;
            }
        }
    };
}

macro_rules! read_default_acl_or_skip {
    ($path:expr) => {
        match PosixACL::read_default_acl($path) {
            Ok(acl) => acl,
            Err(err) => {
                eprintln!("skipping: {}", err);
                return;
            }
        }
    };
}

macro_rules! write_default_acl_or_skip {
    ($acl:expr, $path:expr) => {
        if let Err(err) = $acl.write_default_acl($path) {
            eprintln!("skipping: {}", err);
            return;
        }
    };
}

#[test]
#[serial]
fn daemon_sets_default_acl_on_existing_dir() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let acl_dst_file = read_acl_or_skip!(srv.join("file"));
    assert!(
        acl_dst_file
            .entries()
            .iter()
            .any(|e| e.qualifier() == Qualifier::User(12345) && e.perms().contains(ACL_READ))
    );
}

#[test]
#[serial]
fn daemon_sets_default_acl_on_existing_dir_with_xattrs() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let acl_dst_file = read_acl_or_skip!(srv.join("file"));
    assert!(
        acl_dst_file
            .entries()
            .iter()
            .any(|e| e.qualifier() == Qualifier::User(12345) && e.perms().contains(ACL_READ))
    );
}
