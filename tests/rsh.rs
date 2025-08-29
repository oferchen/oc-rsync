use std::fs;
use tempfile::tempdir;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
mod remote_utils;
#[cfg(unix)]
use remote_utils::{spawn_reader, spawn_writer};
#[cfg(unix)]
use transport::ssh::SshStdioTransport;

#[cfg(unix)]
#[test]
fn rsh_remote_pair_syncs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"via rsh").unwrap();

    let src_session = spawn_reader(&format!("cat {}", src.display()));
    let dst_session = spawn_writer(&format!("cat > {}", dst.display()));
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(&dst).unwrap();
    assert_eq!(out, b"via rsh");
}

#[cfg(unix)]
#[test]
fn custom_rsh_matches_stock_rsync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    fs::write(&src, b"hello shell").unwrap();

    let dst_rr = dir.path().join("dst_rr.txt");
    let dst_rsync = dir.path().join("dst_rsync.txt");

    // Create a fake remote shell that ignores the host argument and executes the rest.
    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    // Use the custom shell with our transport to copy data
    let src_session = SshStdioTransport::spawn(
        rsh.to_str().unwrap(),
        ["ignored", "cat", src.to_str().unwrap()],
    )
    .unwrap();
    let dst_session = SshStdioTransport::spawn(
        rsh.to_str().unwrap(),
        [
            "ignored",
            "sh",
            "-c",
            &format!("cat > {}", dst_rr.display()),
        ],
    )
    .unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Use stock rsync with the same remote shell
    let dst_rsync_spec = format!("ignored:{}", dst_rsync.display());
    let output = Command::new("rsync")
        .args(["-e", rsh.to_str().unwrap(), src.to_str().unwrap(), &dst_rsync_spec])
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let ours = fs::read(&dst_rr).unwrap();
    let theirs = fs::read(&dst_rsync).unwrap();
    assert_eq!(ours, theirs);
}
