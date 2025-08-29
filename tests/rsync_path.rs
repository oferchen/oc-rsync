use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
#[test]
fn custom_rsync_path_performs_transfer() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let src_file = src_dir.join("file.txt");
    fs::write(&src_file, b"from custom binary").unwrap();
    let dst_dir = dir.path().join("dst");

    // Copy the client binary to act as the remote rsync executable.
    let remote_bin = dir.path().join("rr-remote");
    fs::copy(assert_cmd::cargo::cargo_bin("rsync-rs"), &remote_bin).unwrap();
    let mut perms = fs::metadata(&remote_bin).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&remote_bin, perms).unwrap();

    // Create a fake remote shell that ignores the host argument.
    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args([
        "-e",
        rsh.to_str().unwrap(),
        "--rsync-path",
        remote_bin.to_str().unwrap(),
        "-r",
        &src_spec,
        &dst_spec,
    ]);
    cmd.assert().success();

    let out = fs::read(dst_dir.join("file.txt")).unwrap();
    assert_eq!(out, b"from custom binary");
}
