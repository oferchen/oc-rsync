// tests/remote_remote_hard_links.rs
#![cfg(unix)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
#[serial]
fn remote_remote_preserves_hard_links() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(src.join("d1")).unwrap();
    fs::create_dir_all(src.join("d2")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let f1 = src.join("d1/a");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("d2/b");
    fs::hard_link(&f1, &f2).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("fake:{}", src.display());
    let dst_spec = format!("fake:{}", dst.display());

    let rr_bin = cargo_bin("oc-rsync");
    let rr_dir = rr_bin.parent().unwrap();
    let path_env = format!("{}:{}", rr_dir.display(), std::env::var("PATH").unwrap());
    Command::new(&rr_bin)
        .env("PATH", path_env)
        .args([
            "-aH",
            "--rsh",
            rsh.to_str().unwrap(),
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let ino_a = fs::metadata(dst.join("d1/a")).unwrap().ino();
    let ino_b = fs::metadata(dst.join("d2/b")).unwrap().ino();
    assert_eq!(ino_a, ino_b);
}
