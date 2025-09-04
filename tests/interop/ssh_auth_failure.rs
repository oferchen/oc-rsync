// tests/interop/ssh_auth_failure.rs
#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn ssh_auth_failure_matches_rsync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let dst_dir = dir.path().join("dst");
    fs::create_dir(&dst_dir).unwrap();

    let rsh = dir.path().join("deny.sh");
    fs::write(&rsh, b"#!/bin/sh\necho Permission denied >&2\nexit 255\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("localhost:{}/", src_dir.display());
    let dst_spec = dst_dir.to_str().unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-e", rsh.to_str().unwrap(), &src_spec, dst_spec])
        .output()
        .unwrap();
    let upstream = StdCommand::new("rsync")
        .args(["-e", rsh.to_str().unwrap(), &src_spec, dst_spec])
        .output()
        .unwrap();

    assert_eq!(upstream.status.code(), ours.status.code());
    assert_eq!(ours.status.code(), Some(255));
    let our_err = String::from_utf8_lossy(&ours.stderr);
    let up_err = String::from_utf8_lossy(&upstream.stderr);
    assert!(our_err.contains("Permission denied"));
    assert!(our_err.contains("unexplained error (code 255)"));
    assert!(up_err.contains("Permission denied"));
    assert!(up_err.contains("unexplained error (code 255)"));
}
