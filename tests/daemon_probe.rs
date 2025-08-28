use assert_cmd::Command;
use predicates::str::contains;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn daemon_accepts_modules() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["daemon", "--module", "data=/tmp", "--module", "home=/home"]);
    cmd.assert()
        .success()
        .stdout(contains("data => /tmp"))
        .stderr("");
}

#[test]
fn probe_reports_version() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["probe"]);
    cmd.assert()
        .success()
        .stdout(contains("negotiated version"))
        .stderr("");
}

#[test]
fn probe_connects_to_peer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).unwrap();
            stream.write_all(&protocol::LATEST_VERSION.to_be_bytes()).unwrap();
        }
    });

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["probe", &addr.to_string()]);
    cmd.assert()
        .success()
        .stdout(contains("negotiated version"))
        .stderr("");
}

#[test]
fn probe_rejects_old_version() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["probe", "--peer-version", "1"]);
    cmd.assert().failure();
}
