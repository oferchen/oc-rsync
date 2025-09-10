// tests/daemon_xattrs.rs
#![cfg(all(unix, feature = "xattr"))]

use assert_cmd::{Command, cargo::CommandCargoExt};
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

use meta::{acls_supported, xattrs_supported};
mod common;
use common::daemon::{spawn_daemon, spawn_rsync_daemon, wait_for_daemon};

fn try_set_xattr(path: &std::path::Path, name: &str, value: &[u8]) -> bool {
    match xattr::set(path, name, value) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(e) => panic!("setting {name}: {e}"),
    }
}

#[test]
#[serial]
#[cfg(feature = "acl")]
fn daemon_preserves_xattrs() {
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
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    let sec_ok = try_set_xattr(&file, "security.test", b"secret");

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();
    let keep_ok = try_set_xattr(&srv_file, "security.keep", b"dest");

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
    assert!(xattr::get(srv.join("file"), "user.old").unwrap().is_none());
    if sec_ok {
        match xattr::get(srv.join("file"), "security.test") {
            Ok(None) => {}
            Ok(Some(_)) => panic!("security.test should be absent"),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(e) => panic!("get security.test: {e}"),
        }
    }
    if keep_ok {
        if let Ok(Some(keep)) = xattr::get(srv.join("file"), "security.keep") {
            assert_eq!(&keep[..], b"dest");
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_preserves_symlink_xattrs_rr_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    std::os::unix::fs::symlink("file", src.join("link")).unwrap();
    xattr::set(src.join("link"), "user.test", b"val").unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--links",
            "--xattrs",
            &src_arg,
            &format!("rsync://127.0.0.1:{port}/mod"),
        ])
        .assert()
        .success();

    let val = xattr::get(srv.join("link"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_preserves_xattrs_rr_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    let sec_ok = try_set_xattr(&file, "security.test", b"secret");

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();
    let keep_ok = try_set_xattr(&srv_file, "security.keep", b"dest");

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--xattrs",
            &src_arg,
            &format!("rsync://127.0.0.1:{port}/mod"),
        ])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
    assert!(xattr::get(srv.join("file"), "user.old").unwrap().is_none());
    if sec_ok {
        match xattr::get(srv.join("file"), "security.test") {
            Ok(None) => {}
            Ok(Some(_)) => panic!("security.test should be absent"),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(e) => panic!("get security.test: {e}"),
        }
    }
    if keep_ok {
        if let Ok(Some(keep)) = xattr::get(srv.join("file"), "security.keep") {
            assert_eq!(&keep[..], b"dest");
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_preserves_xattrs_rr_daemon() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    let sec_ok = try_set_xattr(&file, "security.test", b"secret");

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();
    let keep_ok = try_set_xattr(&srv_file, "security.keep", b"dest");

    let (mut child, port) = spawn_rsync_daemon(&srv, "  read only = false\n");
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--xattrs",
            &src_arg,
            &format!("rsync://127.0.0.1:{port}/mod"),
        ])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
    assert!(xattr::get(srv.join("file"), "user.old").unwrap().is_none());
    if sec_ok {
        match xattr::get(srv.join("file"), "security.test") {
            Ok(None) => {}
            Ok(Some(_)) => panic!("security.test should be absent"),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
            Err(e) => panic!("get security.test: {e}"),
        }
    }
    if keep_ok {
        if let Ok(Some(keep)) = xattr::get(srv.join("file"), "security.keep") {
            assert_eq!(&keep[..], b"dest");
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
#[cfg(feature = "acl")]
fn daemon_excludes_filtered_xattrs() {
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
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    xattr::set(&file, "user.secret", b"shh").unwrap();

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.secret", b"keep").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-AX",
            "--filter=-x user.secret",
            &src_arg,
            &format!("rsync://127.0.0.1:{port}/mod"),
        ])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
    let secret = xattr::get(srv.join("file"), "user.secret")
        .unwrap()
        .unwrap();
    assert_eq!(&secret[..], b"keep");
    assert!(xattr::get(srv.join("file"), "user.old").unwrap().is_none());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_excludes_filtered_xattrs_rr_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    xattr::set(&file, "user.secret", b"shh").unwrap();

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.secret", b"keep").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--xattrs",
            "--filter=-x user.secret",
            &src_arg,
            &format!("rsync://127.0.0.1:{port}/mod"),
        ])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
    let secret = xattr::get(srv.join("file"), "user.secret")
        .unwrap()
        .unwrap();
    assert_eq!(&secret[..], b"keep");
    assert!(xattr::get(srv.join("file"), "user.old").unwrap().is_none());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_xattrs_match_rsync_server() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();

    let (mut child_oc, port_oc) = spawn_daemon(&srv_oc);
    wait_for_daemon(port_oc);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-aX", &src_arg, &format!("rsync://127.0.0.1:{port_oc}/mod")])
        .assert()
        .success();
    let _ = child_oc.kill();
    let _ = child_oc.wait();

    let (mut child_rs, port_rs) = spawn_rsync_daemon(&srv_rs, "  read only = false\n");
    wait_for_daemon(port_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-aX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();
    let _ = child_rs.kill();
    let _ = child_rs.wait();

    let val_oc = xattr::get(srv_oc.join("file"), "user.test")
        .unwrap()
        .unwrap();
    let val_rs = xattr::get(srv_rs.join("file"), "user.test")
        .unwrap()
        .unwrap();
    assert_eq!(val_oc, val_rs);
}
