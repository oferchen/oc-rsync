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
fn sync_daemon_acls_server() -> (tempfile::TempDir, PathBuf, PathBuf) {
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

    (tmp, srv_oc, srv_rs)
}

#[cfg(feature = "root")]
fn sync_daemon_acls_client() -> (tempfile::TempDir, PathBuf, PathBuf) {
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

    (tmp, srv_oc, srv_rs)
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

    let mut acl = PosixACL::read_acl(&src_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    acl.write_acl(&src_file).unwrap();

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
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

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
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

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
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

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_dst = PosixACL::read_acl(&srv_file).unwrap();
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
    dacl.write_default_acl(&srv).unwrap();

    let daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--acls", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
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
        let mut acl = PosixACL::read_acl(&src_file).unwrap();
        acl.set(Qualifier::User(12345), ACL_READ);
        acl.write_acl(&src_file).unwrap();
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

    let acl_dst = PosixACL::read_acl(srv.join("file")).unwrap();
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
        dacl.write_default_acl(src).unwrap();

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

    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert!(dacl_dst.get(Qualifier::User(12345)).is_none());

    let dacl_dst_sub = PosixACL::read_default_acl(srv.join("sub")).unwrap();
    assert!(dacl_dst_sub.get(Qualifier::User(12345)).is_none());

    let acl_dst_file = PosixACL::read_acl(srv.join("sub/file")).unwrap();
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
    dacl.write_default_acl(&src).unwrap();

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

    let src_dacl = PosixACL::read_default_acl(&src).unwrap();
    let src_sub_dacl = PosixACL::read_default_acl(&sub).unwrap();
    let dst_dacl = PosixACL::read_default_acl(&srv).unwrap();
    let dst_sub_dacl = PosixACL::read_default_acl(srv.join("sub")).unwrap();
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
    dacl.write_default_acl(&src).unwrap();

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

    let acl_src_file = PosixACL::read_acl(&src_file).unwrap();
    let acl_dst_file = PosixACL::read_acl(srv.join("sub/file")).unwrap();
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
    dacl.write_default_acl(&src).unwrap();

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

    let dacl_src = PosixACL::read_default_acl(&src).unwrap();
    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert_eq!(dacl_src.entries(), dacl_dst.entries());

    let dacl_src_sub = PosixACL::read_default_acl(&sub).unwrap();
    let dacl_dst_sub = PosixACL::read_default_acl(srv.join("sub")).unwrap();
    assert_eq!(dacl_src_sub.entries(), dacl_dst_sub.entries());

    let acl_src_file = PosixACL::read_acl(&src_file).unwrap();
    let acl_dst_file = PosixACL::read_acl(srv.join("sub/file")).unwrap();
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
    let (_tmp, srv_oc, srv_rs) = sync_daemon_acls_server();
    let acl_oc = PosixACL::read_acl(srv_oc.join("file")).unwrap();
    let acl_rs = PosixACL::read_acl(srv_rs.join("file")).unwrap();
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
    let (_tmp, srv_oc, srv_rs) = sync_daemon_acls_server();
    let dacl_oc = PosixACL::read_default_acl(&srv_oc).unwrap();
    let dacl_rs = PosixACL::read_default_acl(&srv_rs).unwrap();
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
    let (_tmp, srv_oc, srv_rs) = sync_daemon_acls_client();
    let acl_oc = PosixACL::read_acl(srv_oc.join("file")).unwrap();
    let acl_rs = PosixACL::read_acl(srv_rs.join("file")).unwrap();
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
    let (_tmp, srv_oc, srv_rs) = sync_daemon_acls_client();
    let dacl_oc = PosixACL::read_default_acl(&srv_oc).unwrap();
    let dacl_rs = PosixACL::read_default_acl(&srv_rs).unwrap();
    assert_eq!(dacl_oc.entries(), dacl_rs.entries());
}
