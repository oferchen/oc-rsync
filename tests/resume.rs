// tests/resume.rs

use assert_cmd::prelude::*;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[cfg(unix)]
use assert_cmd::cargo::cargo_bin;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn partial_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'a'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--partial",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    assert!(dst_dir.join("a.partial").exists());

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--local", "--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[test]
fn append_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'b'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    let dest_file = dst_dir.join("a.txt");
    assert!(dest_file.exists());

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--local", "--append", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dest_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn append_verify_restarts_on_mismatch() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'c'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    let dest_file = dst_dir.join("a.txt");
    let mut partial = std::fs::read(&dest_file).unwrap();
    if !partial.is_empty() {
        partial[0] ^= 1;
        std::fs::write(&dest_file, &partial).unwrap();
    }

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args([
        "--local",
        "--append-verify",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dest_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn partial_restarts_on_mismatch() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'd'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--partial",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    let partial_file = dst_dir.join("a.partial");
    assert!(partial_file.exists());
    let mut partial = std::fs::read(&partial_file).unwrap();
    if !partial.is_empty() {
        partial[0] ^= 1;
        std::fs::write(&partial_file, &partial).unwrap();
    }

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--local", "--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[cfg(unix)]
#[test]
fn remote_partial_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'e'; 2_000_000];
    fs::write(src_dir.join("a.txt"), &data).unwrap();
    fs::write(dst_dir.join("a.partial"), &data[..100_000]).unwrap();

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("oc-rsync"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--rsync-path",
            remote_bin.to_str().unwrap(),
            "--partial",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}
