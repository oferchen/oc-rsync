// tests/interop/daemon_auth_failure.rs
#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn daemon_auth_failure_matches_rsync() {
    let dir = tempdir().unwrap();
    let dst = dir.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "foo:pass\n").unwrap();
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let conf = dir.path().join("rsyncd.conf");
    fs::write(
        &conf,
        format!(
            "[mod]\n    path = {}\n    auth users = foo\n    secrets file = {}\n",
            dir.path().display(),
            secrets.display()
        ),
    )
    .unwrap();

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::new("rsync")
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            &port.to_string(),
            "--config",
            conf.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));

    let spec = format!("rsync://foo@127.0.0.1:{}/mod/", port);
    let pass = dir.path().join("pw");
    fs::write(&pass, "wrong\n").unwrap();
    fs::set_permissions(&pass, fs::Permissions::from_mode(0o600)).unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--password-file",
            pass.to_str().unwrap(),
            &spec,
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let upstream = StdCommand::new("rsync")
        .args([
            "--password-file",
            pass.to_str().unwrap(),
            &spec,
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert_eq!(ours.status.code(), upstream.status.code());
    assert_eq!(ours.status.code(), Some(5));
    let our_err = String::from_utf8_lossy(&ours.stderr);
    let up_err = String::from_utf8_lossy(&upstream.stderr);
    assert!(our_err.contains("auth failed"));
    assert!(up_err.contains("auth failed"));
    let _ = child.kill();
    let _ = child.wait();
}
