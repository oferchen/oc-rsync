// tests/interop/refused_option.rs
#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn ssh_refused_remote_option_matches_rsync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let dst_dir = dir.path().join("dst");
    fs::create_dir(&dst_dir).unwrap();

    let rsh = dir.path().join("rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("localhost:{}/", src_dir.display());
    let dst_spec = dst_dir.to_str().unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--remote-option=--bogus",
            &src_spec,
            dst_spec,
        ])
        .output()
        .unwrap();
    let upstream = StdCommand::new("rsync")
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--remote-option=--bogus",
            &src_spec,
            dst_spec,
        ])
        .output()
        .unwrap();

    assert_eq!(upstream.status.code(), ours.status.code());
    let our_stderr = String::from_utf8_lossy(&ours.stderr);
    let up_stderr = String::from_utf8_lossy(&upstream.stderr);
    assert!(our_stderr.contains("on remote machine: --bogus: unknown option"));
    assert!(up_stderr.contains("on remote machine: --bogus: unknown option"));
    assert!(our_stderr.contains("connection unexpectedly closed"));
    assert!(up_stderr.contains("connection unexpectedly closed"));
    assert_eq!(ours.status.code(), Some(12));
}
