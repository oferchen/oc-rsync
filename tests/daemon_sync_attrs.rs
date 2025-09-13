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

    let tmp = tempdir().expect("create temp dir");
    let src = tmp.path().join("src");
    let module = tmp.path().join("mod");
    fs::create_dir_all(&src).expect("create source dir");
    fs::create_dir_all(&module).expect("create module dir");
    let file = src.join("file");
    fs::write(&file, b"hi").expect("write file");
    fchmodat(
        AT_FDCWD,
        &file,
        Mode::from_bits_truncate(0o741),
        FchmodatFlags::NoFollowSymlink,
    )
    .expect("chmod file");
    chown(&file, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))).expect("chown file");

    let mut daemon = spawn_daemon(&module);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .expect("oc-rsync binary")
        .current_dir(&tmp)
        .args(["-a", &src_arg, &format!("rsync://127.0.0.1:{port}/mod")])
        .assert()
        .success();

    let meta = fs::symlink_metadata(module.join("file")).expect("stat file");
    assert_eq!(meta.permissions().mode() & 0o777, 0o741);
    assert_eq!(meta.uid(), 1);
    assert_eq!(meta.gid(), 1);
}

#[test]
#[serial]
fn daemon_preserves_hard_links_rr_client() {
    use std::os::unix::fs::MetadataExt;
    let tmp = tempdir().expect("create temp dir");
    let src = tmp.path().join("src");
    let module = tmp.path().join("mod");
    fs::create_dir_all(&src).expect("create source dir");
    fs::create_dir_all(&module).expect("create module dir");
    let f1 = src.join("a");
    fs::write(&f1, b"hi").expect("write a");
    let f2 = src.join("b");
    fs::hard_link(&f1, &f2).expect("link b to a");
    let mut daemon = spawn_daemon(&module);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);
    let src_arg = format!("{}/", src.display());
    let dest = format!("rsync://127.0.0.1:{port}/mod");
    Command::cargo_bin("oc-rsync")
        .expect("oc-rsync binary")
        .current_dir(&tmp)
        .args(["-aH", &src_arg, &dest])
        .assert()
        .success();
    let meta1 = fs::symlink_metadata(module.join("a")).expect("stat a");
    let meta2 = fs::symlink_metadata(module.join("b")).expect("stat b");
    assert_eq!(meta1.ino(), meta2.ino());
}
