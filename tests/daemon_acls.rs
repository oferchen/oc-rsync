// tests/daemon_acls.rs
#![cfg(all(unix, feature = "acl"))]

use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

use meta::{acls_supported, xattrs_supported};
use posix_acl::{ACL_READ, ACL_WRITE, PosixACL, Qualifier};
mod common;
#[cfg(feature = "root")]
use common::daemon::spawn_rsync_daemon;
use common::daemon::{spawn_daemon, wait_for_daemon};

#[test]
#[serial]
fn daemon_preserves_file_acls() {
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

    let mut acl = PosixACL::read_acl(&src_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    acl.write_acl(&src_file).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = PosixACL::read_acl(&src_file).unwrap();
    let acl_dst = PosixACL::read_acl(srv.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_preserves_default_acls() {
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
    dacl.write_default_acl(&src).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = PosixACL::read_default_acl(&src).unwrap();
    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_preserves_acls_rr_client() {
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

    let mut acl = PosixACL::read_acl(&src_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    acl.write_acl(&src_file).unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&src).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = PosixACL::read_acl(&src_file).unwrap();
    let acl_dst = PosixACL::read_acl(srv.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let dacl_src = PosixACL::read_default_acl(&src).unwrap();
    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_removes_file_acls() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let _src_file = src.join("file");
    fs::write(&_src_file, b"hi").unwrap();
    let srv_file = srv.join("file");
    fs::write(&srv_file, b"hi").unwrap();

    let mut acl = PosixACL::read_acl(&srv_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&srv_file).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = PosixACL::read_acl(&srv_file).unwrap();
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_removes_default_acls() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let _src_file = src.join("file");
    fs::write(&_src_file, b"hi").unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&srv).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert!(dacl_dst.entries().is_empty());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_ignores_acls_without_flag() {
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

    let mut acl = PosixACL::read_acl(&src_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&src_file).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = PosixACL::read_acl(srv.join("file")).unwrap();
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_inherits_default_acls() {
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

    let mut dacl = PosixACL::read_default_acl(&src).unwrap();
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&src).unwrap();

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let src_file = sub.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = PosixACL::read_default_acl(&src).unwrap();
    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let dacl_src_sub = PosixACL::read_default_acl(&sub).unwrap();
    let dacl_dst_sub = PosixACL::read_default_acl(srv.join("sub")).unwrap();
    assert_eq!(dacl_src_sub.entries(), dacl_dst_sub.entries());

    let acl_src_file = PosixACL::read_acl(&src_file).unwrap();
    let acl_dst_file = PosixACL::read_acl(srv.join("sub/file")).unwrap();
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_inherits_default_acls_rr_client() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();

    let mut dacl = PosixACL::read_default_acl(&src).unwrap();
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&src).unwrap();

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let src_file = sub.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = PosixACL::read_default_acl(&src).unwrap();
    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let dacl_src_sub = PosixACL::read_default_acl(&sub).unwrap();
    let dacl_dst_sub = PosixACL::read_default_acl(srv.join("sub")).unwrap();
    assert_eq!(dacl_src_sub.entries(), dacl_dst_sub.entries());

    let acl_src_file = PosixACL::read_acl(&src_file).unwrap();
    let acl_dst_file = PosixACL::read_acl(srv.join("sub/file")).unwrap();
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_acls_match_rsync_server() {
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
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();

    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = PosixACL::read_acl(&src_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    acl.write_acl(&src_file).unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&src).unwrap();

    let (mut child_oc, port_oc) = spawn_daemon(&srv_oc);
    wait_for_daemon(port_oc);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_oc}/mod")])
        .assert()
        .success();
    let _ = child_oc.kill();
    let _ = child_oc.wait();

    let (mut child_rs, port_rs) = spawn_rsync_daemon(&srv_rs, "");
    wait_for_daemon(port_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();
    let _ = child_rs.kill();
    let _ = child_rs.wait();

    let acl_oc = PosixACL::read_acl(srv_oc.join("file")).unwrap();
    let acl_rs = PosixACL::read_acl(srv_rs.join("file")).unwrap();
    assert_eq!(acl_oc.entries(), acl_rs.entries());

    let dacl_oc = PosixACL::read_default_acl(&srv_oc).unwrap();
    let dacl_rs = PosixACL::read_default_acl(&srv_rs).unwrap();
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}

#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_acls_match_rsync_client() {
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
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();

    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = PosixACL::read_acl(&src_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    acl.write_acl(&src_file).unwrap();

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&src).unwrap();

    let (mut child_oc, port_oc) = spawn_rsync_daemon(&srv_oc, "");
    wait_for_daemon(port_oc);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--acls",
            &src_arg,
            &format!("rsync://127.0.0.1:{port_oc}/mod"),
        ])
        .assert()
        .success();
    let _ = child_oc.kill();
    let _ = child_oc.wait();

    let (mut child_rs, port_rs) = spawn_rsync_daemon(&srv_rs, "");
    wait_for_daemon(port_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();
    let _ = child_rs.kill();
    let _ = child_rs.wait();

    let acl_oc = PosixACL::read_acl(srv_oc.join("file")).unwrap();
    let acl_rs = PosixACL::read_acl(srv_rs.join("file")).unwrap();
    assert_eq!(acl_oc.entries(), acl_rs.entries());

    let dacl_oc = PosixACL::read_default_acl(&srv_oc).unwrap();
    let dacl_rs = PosixACL::read_default_acl(&srv_rs).unwrap();
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}
