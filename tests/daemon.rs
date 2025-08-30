use assert_cmd::prelude::*;
use assert_cmd::Command;
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use transport::{TcpTransport, Transport};

fn read_port(child: &mut Child) -> u16 {
    let stdout = child.stdout.as_mut().unwrap();
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while stdout.read(&mut byte).unwrap() == 1 {
        if byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
    }
    String::from_utf8(buf).unwrap().trim().parse().unwrap()
}

fn spawn_daemon() -> (Child, u16) {
    let mut child = StdCommand::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--daemon", "--module", "data=/tmp", "--port", "0"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
    (child, port)
}

fn spawn_temp_daemon() -> (Child, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            "0",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
    (child, port, dir)
}

fn spawn_daemon_with_address(addr: &str) -> (Child, u16) {
    let mut child = StdCommand::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            "data=/tmp",
            "--port",
            "0",
            "--address",
            addr,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
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
    assert_ne!(port, 873);
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
fn daemon_binds_to_specified_address() {
    let (mut child, port) = spawn_daemon_with_address("127.0.0.1");
    wait_for_daemon(port);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    assert!(TcpStream::connect(("127.0.0.2", port)).is_err());
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
fn daemon_accepts_connection_on_ephemeral_port() {
    let (mut child, port, _dir) = spawn_temp_daemon();
    wait_for_daemon(port);
    TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    let _ = child.kill();
    let _ = child.wait();
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
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
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
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
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
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
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
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
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

#[test]
#[serial]
fn client_respects_no_motd() {
    let dir = tempfile::tempdir().unwrap();
    let motd = dir.path().join("motd");
    fs::write(&motd, "Hello world\n").unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let dst = dir.path().join("dst");
    fs::create_dir(&dst).unwrap();

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
            &format!("data={}", src.display()),
            "--port",
            &port.to_string(),
            "--motd",
            motd.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    wait_for_daemon(port);

    let output = Command::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            &format!("rsync://127.0.0.1:{port}/data/"),
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("Hello world"));

    let output = Command::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--no-motd",
            &format!("rsync://127.0.0.1:{port}/data/"),
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Hello world"));

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_logs_connections() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("log");
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
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format",
            "%h %m",
        ])
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    {
        let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
        t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
        let mut buf = [0u8; 4];
        t.receive(&mut buf).unwrap();
        t.send(b"data\n").unwrap();
    }
    sleep(Duration::from_millis(100));
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("127.0.0.1 data"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_honors_bwlimit() {
    let dir = tempfile::tempdir().unwrap();
    let motd = dir.path().join("motd");
    let line = "A".repeat(256);
    fs::write(&motd, format!("{line}\nsecond")).unwrap();
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
            "--bwlimit",
            "256",
            "--motd",
            motd.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    let mut first = vec![0u8; 300];
    let n1 = t.receive(&mut first).unwrap();
    assert!(!String::from_utf8_lossy(&first[..n1]).contains("second"));
    let start = Instant::now();
    let mut second = [0u8; 64];
    let _ = t.receive(&mut second).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(800));
    let _ = child.kill();
    let _ = child.wait();
}
