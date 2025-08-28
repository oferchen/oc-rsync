use assert_cmd::prelude::*;
use assert_cmd::Command;
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command as StdCommand};
use std::thread::sleep;
use std::time::Duration;

fn spawn_daemon() -> (Child, u16) {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = StdCommand::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            "data=/tmp",
            "--port",
            &port.to_string(),
        ])
        .spawn()
        .unwrap();
    (child, port)
}

fn wait_for_daemon(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}

#[test]
#[serial]
fn daemon_negotiates_version_with_client() {
    let (mut child, port) = spawn_daemon();
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn probe_connects_to_daemon() {
    let (mut child, port) = spawn_daemon();
    wait_for_daemon(port);
    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--probe", &format!("127.0.0.1:{port}")])
        .assert()
        .success();
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn probe_rejects_old_version() {
    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--probe", "--peer-version", "1"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn daemon_rejects_unauthorized_client() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("auth"), "secret data\n").unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            &port.to_string(),
        ])
        .current_dir(dir.path())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    stream.write_all(b"bad\n").unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let n = stream.read(&mut buf).unwrap_or(0);
    assert!(n == 0 || String::from_utf8_lossy(&buf[..n]).starts_with("@ERROR"),);
    let _ = child.kill();
    let _ = child.wait();
}
