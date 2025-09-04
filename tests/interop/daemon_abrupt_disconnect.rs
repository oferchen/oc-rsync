// tests/interop/daemon_abrupt_disconnect.rs
#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;
use tempfile::tempdir;

#[test]
fn daemon_connection_drop_matches_rsync() {
    let dir = tempdir().unwrap();
    let dst = dir.path().join("dst");
    fs::create_dir(&dst).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            drop(stream);
        }
    });

    let spec = format!("rsync://127.0.0.1:{}/mod/", port);
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&spec, dst.to_str().unwrap()])
        .output()
        .unwrap();
    let upstream = StdCommand::new("rsync")
        .args([&spec, dst.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(ours.status.code(), upstream.status.code());
    assert_eq!(ours.status.code(), Some(12));
    let our_err = String::from_utf8_lossy(&ours.stderr);
    let up_err = String::from_utf8_lossy(&upstream.stderr);
    assert!(our_err.contains("Connection reset by peer") || our_err.contains("connection unexpectedly closed"));
    assert!(up_err.contains("Connection reset by peer"));
}
