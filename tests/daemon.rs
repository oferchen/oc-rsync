use assert_cmd::prelude::*;
use assert_cmd::Command;
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command as StdCommand};
use std::thread::sleep;
use std::time::Duration;
use transport::{TcpTransport, Transport};

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
fn daemon_rejects_invalid_token() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
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
            "--secrets-file",
            secrets.to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect(&format!("127.0.0.1:{port}")).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("bad")).unwrap();
    t.set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || String::from_utf8_lossy(&buf[..n]).starts_with("@ERR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_rejects_unauthorized_module() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret other\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
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
            "--secrets-file",
            secrets.to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect(&format!("127.0.0.1:{port}")).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("secret")).unwrap();
    t.send(b"data\n").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || String::from_utf8_lossy(&buf[..n]).starts_with("@ERR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_accepts_authorized_client() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
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
            "--secrets-file",
            secrets.to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect(&format!("127.0.0.1:{port}")).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("secret")).unwrap();
    t.send(b"data\n").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let res = t.receive(&mut buf);
    match res {
        Ok(0) => {}
        Ok(n) => assert!(!String::from_utf8_lossy(&buf[..n]).starts_with("@ERROR")),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {}
        Err(e) => panic!("unexpected error: {e}"),
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_respects_host_allow_and_deny_lists() {
    // Allow list
    let (mut child, port) = {
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
                "--hosts-allow",
                "127.0.0.1",
            ])
            .spawn()
            .unwrap();
        (child, port)
    };
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    let _ = child.kill();
    let _ = child.wait();

    // Deny list
    let (mut child, port) = {
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
                "--hosts-deny",
                "127.0.0.1",
            ])
            .spawn()
            .unwrap();
        (child, port)
    };
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    let res = stream.read(&mut buf);
    assert!(res.is_err() || res.unwrap() == 0);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_displays_motd() {
    let dir = tempfile::tempdir().unwrap();
    let motd = dir.path().join("motd");
    fs::write(&motd, "Hello world\n").unwrap();
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
            "data=/tmp",
            "--port",
            &port.to_string(),
            "--motd",
            motd.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect(&format!("127.0.0.1:{port}")).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    t.authenticate(None).unwrap();
    let mut motd_buf = [0u8; 64];
    let n = t.receive(&mut motd_buf).unwrap();
    assert!(String::from_utf8_lossy(&motd_buf[..n]).contains("Hello world"));
    let _ = child.kill();
    let _ = child.wait();
}
