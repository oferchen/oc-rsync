// tests/daemon_sync_attrs.rs
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use assert_cmd::{cargo::CommandCargoExt, Command};
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use serial_test::serial;
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use std::fs;
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use std::net::{TcpListener, TcpStream};
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use std::process::{Child, Command as StdCommand};
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use std::thread::sleep;
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use std::time::Duration;
#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
use tempfile::tempdir;

#[cfg(all(unix, feature = "acl"))]
use posix_acl::{PosixACL, Qualifier, ACL_READ};
#[cfg(all(unix, feature = "xattr"))]
use xattr;

#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
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

#[cfg(all(unix, any(feature = "xattr", feature = "acl")))]
fn wait_for_daemon(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}

#[cfg(all(unix, feature = "xattr"))]
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

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::new("rsync")
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(all(unix, feature = "acl"))]
#[test]
#[ignore]
#[serial]
fn daemon_preserves_acls() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();

    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&file).unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::new("rsync")
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(&srv.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let _ = child.kill();
    let _ = child.wait();
}
