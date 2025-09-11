// tests/daemon_sync_attrs.rs
#![cfg(unix)]

use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

mod common;
use common::daemon::{spawn_daemon, wait_for_daemon};

#[cfg(feature = "root")]
#[test]
#[serial]
#[cfg_attr(not(target_os = "linux"), ignore = "requires Linux uid/gid semantics")]
#[ignore = "requires root privileges"]
fn daemon_preserves_uid_gid_perms() {
    use nix::fcntl::AT_FDCWD;
    use nix::sys::stat::{FchmodatFlags, Mode, fchmodat};
    use nix::unistd::{Gid, Uid, chown};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    assert!(Uid::effective().is_root(), "requires root privileges");

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fchmodat(
        AT_FDCWD,
        &file,
        Mode::from_bits_truncate(0o741),
        FchmodatFlags::NoFollowSymlink,
    )
    .unwrap();
    chown(&file, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))).unwrap();

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

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
}

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
    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);
    let src_arg = format!("{}/", src.display());
    let dest = format!("rsync://127.0.0.1:{port}/mod");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-aH", &src_arg, &dest])
        .assert()
        .success();
    let meta1 = fs::symlink_metadata(srv.join("a")).unwrap();
    let meta2 = fs::symlink_metadata(srv.join("b")).unwrap();
    assert_eq!(meta1.ino(), meta2.ino());
}
