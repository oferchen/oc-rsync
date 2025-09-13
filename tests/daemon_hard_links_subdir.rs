// tests/daemon_hard_links_subdir.rs
use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

mod common;
use common::daemon::{spawn_daemon, wait_for_daemon};

#[cfg(unix)]
#[test]
#[serial]
fn daemon_preserves_hard_links_in_subdir() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();

    let f1 = src.join("a");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("b");
    fs::hard_link(&f1, &f2).unwrap();
    let f3 = src.join("c");
    fs::hard_link(&f1, &f3).unwrap();

    let mut daemon = spawn_daemon(&srv);
    let port = daemon.port;
    wait_for_daemon(&mut daemon);

    let src_arg = format!("{}/", src.display());
    let dest = format!("rsync://127.0.0.1:{port}/mod/sub/");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-aH", &src_arg, &dest])
        .assert()
        .success();

    let sub = srv.join("sub");
    let ino_a = fs::metadata(sub.join("a")).unwrap().ino();
    let ino_b = fs::metadata(sub.join("b")).unwrap().ino();
    let ino_c = fs::metadata(sub.join("c")).unwrap().ino();
    assert_eq!(ino_a, ino_b);
    assert_eq!(ino_a, ino_c);
}
