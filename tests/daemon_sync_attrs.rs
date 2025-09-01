// tests/daemon_sync_attrs.rs

#[cfg(unix)]
use assert_cmd::{cargo::CommandCargoExt, Command};
#[cfg(unix)]
use serial_test::serial;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::process::{Child, Command as StdCommand};
#[cfg(unix)]
use std::thread::sleep;
#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tempfile::tempdir;

#[cfg(all(unix, feature = "acl"))]
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
    xattr::set(&file, "security.test", b"secret").unwrap();

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();
    xattr::set(&srv_file, "security.keep", b"dest").unwrap();

    let (mut child, port) = spawn_daemon(&srv);
    wait_for_daemon(port);

    let src_arg = format!("{}/", src.display());
    Command::new("rsync")
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let val = xattr::get(srv.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
    assert!(xattr::get(srv.join("file"), "user.old").unwrap().is_none());
    assert!(xattr::get(srv.join("file"), "security.test")
        .unwrap()
        .is_none());
    let keep = xattr::get(srv.join("file"), "security.keep")
        .unwrap()
        .unwrap();
    assert_eq!(&keep[..], b"dest");

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(all(unix, feature = "xattr"))]
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
    xattr::set(&file, "security.test", b"secret").unwrap();

    let srv_file = srv.join("file");
    fs::write(&srv_file, b"old").unwrap();
    xattr::set(&srv_file, "user.old", b"junk").unwrap();
    xattr::set(&srv_file, "security.keep", b"dest").unwrap();

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
    assert!(xattr::get(srv.join("file"), "security.test")
        .unwrap()
        .is_none());
    let keep = xattr::get(srv.join("file"), "security.keep")
        .unwrap()
        .unwrap();
    assert_eq!(&keep[..], b"dest");

    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(all(unix, feature = "xattr"))]
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
    Command::new("rsync")
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

#[cfg(all(unix, feature = "xattr"))]
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

#[cfg(all(unix, feature = "acl"))]
#[test]
#[serial]
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
    Command::new("rsync")
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

#[cfg(all(unix, feature = "acl"))]
#[test]
#[serial]
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

#[cfg(all(unix, feature = "acl"))]
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
    Command::new("rsync")
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

#[cfg(all(unix, feature = "acl"))]
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
fn daemon_preserves_uid_gid_perms() {
    use nix::sys::stat::{fchmodat, FchmodatFlags, Mode};
    use nix::unistd::{chown, Gid, Uid};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

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
    Command::new("rsync")
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
