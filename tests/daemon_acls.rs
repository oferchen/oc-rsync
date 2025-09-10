// tests/daemon_acls.rs
#![cfg(all(unix, feature = "acl"))]

use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

use meta::{acls_supported, xattrs_supported};
use posix_acl::{ACL_READ, ACL_WRITE, PosixACL, Qualifier};
mod common;
#[cfg(feature = "root")]
use common::daemon::spawn_rsync_daemon;
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

macro_rules! write_acl_or_skip {
    ($acl:expr, $path:expr) => {
        if let Err(err) = $acl.write_acl($path) {
            eprintln!("skipping: {}", err);
            return;
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

fn setup_acl_dirs<F>(populate: F) -> (tempfile::TempDir, PathBuf, PathBuf)
where
    F: FnOnce(&Path),
{
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    populate(&src);
    (tmp, src, srv)
}

#[cfg(feature = "root")]
fn sync_daemon_acls_server() -> Option<(tempfile::TempDir, PathBuf, PathBuf)> {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();

    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = match PosixACL::read_acl(&src_file) {
        Ok(acl) => acl,
        Err(err) => {
            eprintln!("skipping: {}", err);
            return None;
        }
    };
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    if let Err(err) = acl.write_acl(&src_file) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    if let Err(err) = dacl.write_default_acl(&src) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let daemon_oc = spawn_daemon(&srv_oc);
    let port_oc = daemon_oc.port;
    wait_for_daemon(port_oc);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_oc}/mod")])
        .assert()
        .success();
    let daemon_rs = spawn_rsync_daemon(&srv_rs, "");
    let port_rs = daemon_rs.port;
    wait_for_daemon(port_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();

    Some((tmp, srv_oc, srv_rs))
}

#[cfg(feature = "root")]
fn sync_daemon_acls_client() -> Option<(tempfile::TempDir, PathBuf, PathBuf)> {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();

    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = match PosixACL::read_acl(&src_file) {
        Ok(acl) => acl,
        Err(err) => {
            eprintln!("skipping: {}", err);
            return None;
        }
    };
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    if let Err(err) = acl.write_acl(&src_file) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    if let Err(err) = dacl.write_default_acl(&src) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let daemon_oc = spawn_rsync_daemon(&srv_oc, "");
    let port_oc = daemon_oc.port;
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
    let daemon_rs = spawn_rsync_daemon(&srv_rs, "");
    let port_rs = daemon_rs.port;
    wait_for_daemon(port_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();

    Some((tmp, srv_oc, srv_rs))
}

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

    let mut acl = read_acl_or_skip!(&src_file);
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    write_acl_or_skip!(acl, &src_file);

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = read_acl_or_skip!(&src_file);
    let acl_dst = read_acl_or_skip!(srv.join("file"));
    assert_eq!(acl_src.entries(), acl_dst.entries());
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
    write_default_acl_or_skip!(dacl, &src);

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
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

    let mut acl = read_acl_or_skip!(&src_file);
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    write_acl_or_skip!(acl, &src_file);

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = read_acl_or_skip!(&src_file);
    let acl_dst = read_acl_or_skip!(srv.join("file"));
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
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

    let mut acl = read_acl_or_skip!(&srv_file);
    acl.set(Qualifier::User(12345), ACL_READ);
    write_acl_or_skip!(acl, &srv_file);

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = read_acl_or_skip!(&srv_file);
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());
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
    write_default_acl_or_skip!(dacl, &srv);

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert!(dacl_dst.entries().is_empty());
}

#[test]
#[serial]
fn daemon_ignores_acls_without_flag() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let (_tmp, src, srv) = setup_acl_dirs(|src| {
        let src_file = src.join("file");
        fs::write(&src_file, b"hi").unwrap();
        let mut acl = read_acl_or_skip!(&src_file);
        acl.set(Qualifier::User(12345), ACL_READ);
        write_acl_or_skip!(acl, &src_file);
    });

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = read_acl_or_skip!(srv.join("file"));
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());
}

#[test]
#[serial]
fn daemon_ignores_default_acls_without_flag() {
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let (_tmp, src, srv) = setup_acl_dirs(|src| {
        let mut dacl = PosixACL::new(0o755);
        dacl.set(Qualifier::User(12345), ACL_READ);
        write_default_acl_or_skip!(dacl, src);

        let sub = src.join("sub");
        fs::create_dir(&sub).unwrap();
        let src_file = sub.join("file");
        fs::write(&src_file, b"hi").unwrap();
    });

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert!(dacl_dst.get(Qualifier::User(12345)).is_none());

    let dacl_dst_sub = read_default_acl_or_skip!(srv.join("sub"));
    assert!(dacl_dst_sub.get(Qualifier::User(12345)).is_none());

    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert!(acl_dst_file.get(Qualifier::User(12345)).is_none());
}

#[test]
#[serial]
fn daemon_inherits_directory_default_acls() {
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

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let src_dacl = read_default_acl_or_skip!(&src);
    let src_sub_dacl = read_default_acl_or_skip!(&sub);
    let dst_dacl = read_default_acl_or_skip!(&srv);
    let dst_sub_dacl = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(
        (src_dacl.entries(), src_sub_dacl.entries()),
        (dst_dacl.entries(), dst_sub_dacl.entries())
    );
}

#[test]
#[serial]
fn daemon_inherits_file_acls_from_default() {
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

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let src_file = sub.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src_file = read_acl_or_skip!(&src_file);
    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());
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

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    write_default_acl_or_skip!(dacl, &src);

    let sub = src.join("sub");
    fs::create_dir(&sub).unwrap();
    let src_file = sub.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_src = read_default_acl_or_skip!(&src);
    let dacl_dst = read_default_acl_or_skip!(&srv);
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let dacl_src_sub = read_default_acl_or_skip!(&sub);
    let dacl_dst_sub = read_default_acl_or_skip!(srv.join("sub"));
    assert_eq!(dacl_src_sub.entries(), dacl_dst_sub.entries());

    let acl_src_file = read_acl_or_skip!(&src_file);
    let acl_dst_file = read_acl_or_skip!(srv.join("sub/file"));
    assert_eq!(acl_src_file.entries(), acl_dst_file.entries());
}

#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_file_acls_match_rsync_server() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_server() else {
        return;
    };
    let acl_oc = read_acl_or_skip!(srv_oc.join("file"));
    let acl_rs = read_acl_or_skip!(srv_rs.join("file"));
    assert_eq!(acl_oc.entries(), acl_rs.entries());
}

#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_default_acls_match_rsync_server() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_server() else {
        return;
    };
    let dacl_oc = read_default_acl_or_skip!(&srv_oc);
    let dacl_rs = read_default_acl_or_skip!(&srv_rs);
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}

#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_file_acls_match_rsync_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_client() else {
        return;
    };
    let acl_oc = read_acl_or_skip!(srv_oc.join("file"));
    let acl_rs = read_acl_or_skip!(srv_rs.join("file"));
    assert_eq!(acl_oc.entries(), acl_rs.entries());
}

#[cfg(feature = "root")]
#[test]
#[serial]
fn daemon_default_acls_match_rsync_client() {
    if !xattrs_supported() {
        eprintln!("skipping: xattrs unsupported");
        return;
    }
    if !acls_supported() {
        eprintln!("skipping: ACLs unsupported");
        return;
    }
    let Some((_tmp, srv_oc, srv_rs)) = sync_daemon_acls_client() else {
        return;
    };
    let dacl_oc = read_default_acl_or_skip!(&srv_oc);
    let dacl_rs = read_default_acl_or_skip!(&srv_rs);
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}
