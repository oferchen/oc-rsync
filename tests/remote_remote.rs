use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
#[ignore]
fn remote_to_remote_pipes_data() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"hello remote\n").unwrap();

    let src_script = dir.path().join("src.sh");
    fs::write(
        &src_script,
        format!("#!/bin/sh\ncat {}\n", src_file.display()),
    )
    .unwrap();

    let dst_script = dir.path().join("dst.sh");
    fs::write(
        &dst_script,
        format!("#!/bin/sh\ncat > {}\n", dst_file.display()),
    )
    .unwrap();

    let src_spec = format!("sh:{}", src_script.to_str().unwrap());
    let dst_spec = format!("sh:{}", dst_script.to_str().unwrap());

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args([&src_spec, &dst_spec]);
    cmd.assert().success();

    let out = fs::read(&dst_file).unwrap();
    assert_eq!(out, b"hello remote\n");
}

#[test]
fn remote_pair_missing_host_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    // Missing host in source spec should yield an error before attempting connections
    cmd.args([":/tmp/src", "sh:/tmp/dst"]);
    cmd.assert().failure();
}

#[test]
fn remote_pair_missing_path_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    // Missing path in source spec should also fail
    cmd.args(["sh:", "sh:/tmp/dst"]);
    cmd.assert().failure();
}
