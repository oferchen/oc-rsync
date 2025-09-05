// tests/daemon_sync_attrs.rs
#![cfg(all(unix, feature = "acl"))]

use assert_cmd::{
    cargo::{cargo_bin, CommandCargoExt},
    Command,
};
use serial_test::serial;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command as StdCommand};
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;

use posix_acl::{PosixACL, Qualifier, ACL_READ, ACL_WRITE};

#[cfg(unix)]
fn spawn_daemon(root: &std::path::Path) -> (Child, u16) {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("mod={}", root.display()),
            "--port",
            &port.to_string(),
        ])
        .spawn()
        .unwrap();
    (child, port)
}

#[cfg(unix)]
fn spawn_rsync_daemon_acl(root: &std::path::Path) -> (Child, u16) {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let conf = format!(
        "uid = 0\ngid = 0\nuse chroot = false\n[mod]\n    path = {}\n",
        root.display()
    );
    let conf_path = root.join("rsyncd.conf");
    fs::write(&conf_path, conf).unwrap();
    let child = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            &port.to_string(),
            "--config",
            conf_path.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    (child, port)
}

#[cfg(unix)]
fn wait_for_daemon(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}

#[cfg(unix)]
fn try_set_xattr(path: &std::path::Path, name: &str, value: &[u8]) -> bool {
    match xattr::set(path, name, value) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(e) => panic!("setting {name}: {e}"),
    }
}

#[cfg(unix)]
fn spawn_rsync_daemon_xattr(root: &std::path::Path) -> (Child, u16) {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let conf = root.join("rsyncd.conf");
    fs::write(
        &conf,
        format!(
            "uid = 0\ngid = 0\nuse chroot = false\n[mod]\n  path = {}\n  read only = false\n",
            root.display()
        ),
    )
    .unwrap();
    let child = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            &port.to_string(),
            "--config",
            conf.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    (child, port)
}

#[cfg(unix)]
#[test]
#[ignore]
#[serial]
fn daemon_preserves_xattrs() {
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

#[cfg(unix)]
#[test]
#[serial]
fn daemon_preserves_symlink_xattrs_rr_client() {
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

#[cfg(unix)]
#[test]
#[serial]
fn daemon_preserves_xattrs_rr_client() {
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

#[cfg(unix)]
#[test]
#[ignore]
#[serial]
fn daemon_preserves_xattrs_rr_daemon() {
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

    let (mut child, port) = spawn_rsync_daemon_xattr(&srv);
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

#[cfg(unix)]
#[test]
#[ignore]
#[serial]
fn daemon_excludes_filtered_xattrs() {
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

#[cfg(unix)]
#[test]
#[ignore]
#[serial]
fn daemon_excludes_filtered_xattrs_rr_client() {
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

#[cfg(unix)]
#[test]
#[serial]
fn daemon_xattrs_match_rsync_server() {
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

    let (mut child_rs, port_rs) = spawn_rsync_daemon_xattr(&srv_rs);
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

#[cfg(unix)]
#[test]
#[serial]
#[cfg_attr(not(target_os = "linux"), ignore = "requires Linux ACLs")]
fn daemon_preserves_acls() {
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
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
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

#[cfg(unix)]
#[test]
#[serial]
#[cfg_attr(not(target_os = "linux"), ignore = "requires Linux ACLs")]
fn daemon_preserves_acls_rr_client() {
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

#[cfg(unix)]
#[test]
#[serial]
#[cfg_attr(not(target_os = "linux"), ignore = "requires Linux ACLs")]
fn daemon_removes_acls() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();
    let srv_file = srv.join("file");
    fs::write(&srv_file, b"hi").unwrap();

    let mut acl = PosixACL::read_acl(&srv_file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&srv_file).unwrap();
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

    let acl_dst = PosixACL::read_acl(&srv_file).unwrap();
    assert!(acl_dst.get(Qualifier::User(12345)).is_none());

    let dacl_dst = PosixACL::read_default_acl(&srv).unwrap();
    assert!(dacl_dst.entries().is_empty());

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[test]
#[serial]
fn daemon_ignores_acls_without_flag() {
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

#[cfg(unix)]
#[test]
#[serial]
fn daemon_inherits_default_acls() {
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

#[cfg(unix)]
#[test]
#[serial]
fn daemon_inherits_default_acls_rr_client() {
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

#[cfg(unix)]
#[test]
#[serial]
fn daemon_acls_match_rsync_server() {
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

    let (mut child_rs, port_rs) = spawn_rsync_daemon_acl(&srv_rs);
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

#[cfg(unix)]
#[test]
#[serial]
#[cfg_attr(not(target_os = "linux"), ignore = "requires Linux ACLs")]
fn daemon_acls_match_rsync_client() {
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

    let (mut child_oc, port_oc) = spawn_rsync_daemon_acl(&srv_oc);
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

    let (mut child_rs, port_rs) = spawn_rsync_daemon_acl(&srv_rs);
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

#[cfg(unix)]
#[test]
#[serial]
#[cfg_attr(not(target_os = "linux"), ignore = "requires Linux uid/gid semantics")]
fn daemon_preserves_uid_gid_perms() {
    use nix::sys::stat::{fchmodat, FchmodatFlags, Mode};
    use nix::unistd::{chown, Gid, Uid};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    if !Uid::effective().is_root() {
        eprintln!("skipping: requires root privileges");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fchmodat(
        None,
        &file,
        Mode::from_bits_truncate(0o741),
        FchmodatFlags::NoFollowSymlink,
    )
    .unwrap();
    chown(&file, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-a", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let meta = fs::symlink_metadata(srv.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o741);
    assert_eq!(meta.uid(), 1);
    assert_eq!(meta.gid(), 1);

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
#[test]
#[serial]
fn daemon_preserves_hard_links_rr_client() {
    use std::os::unix::fs::MetadataExt;
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let f1 = src.join("a");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("b");
    fs::hard_link(&f1, &f2).unwrap();
    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-aH", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();
    let meta1 = fs::symlink_metadata(srv.join("a")).unwrap();
    let meta2 = fs::symlink_metadata(srv.join("b")).unwrap();
    assert_eq!(meta1.ino(), meta2.ino());
    let _ = child.kill();
    let _ = child.wait();
}
