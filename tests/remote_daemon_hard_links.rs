// tests/remote_daemon_hard_links.rs
#![cfg(unix)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

mod common;
use common::daemon::{spawn_daemon, wait_for_daemon};

#[test]
#[serial]
fn remote_source_to_daemon_preserves_hard_links_without_local_dirs() {
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

    let rsh = tmp.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_arg = format!("fake:{}/", src.display());
    let dest = format!("rsync://127.0.0.1:{port}/mod/");

    let rr_bin = cargo_bin("oc-rsync");
    let rr_dir = rr_bin.parent().unwrap();
    let path_env = format!("{}:{}", rr_dir.display(), std::env::var("PATH").unwrap());
    Command::new(rr_bin)
        .env("PATH", path_env)
        .current_dir(&tmp)
        .args(["-aH", "--rsh", rsh.to_str().unwrap(), &src_arg, &dest])
        .assert()
        .success();

    assert!(!tmp.path().join("rsync:").exists());

    let ino_a = fs::metadata(srv.join("a")).unwrap().ino();
    let ino_b = fs::metadata(srv.join("b")).unwrap().ino();
    let ino_c = fs::metadata(srv.join("c")).unwrap().ino();
    assert_eq!(ino_a, ino_b);
    assert_eq!(ino_a, ino_c);
}
