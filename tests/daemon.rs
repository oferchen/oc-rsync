// tests/daemon.rs

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
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};
use transport::{TcpTransport, Transport};
use wait_timeout::ChildExt;

struct Skip;

fn require_network() -> Result<(), Skip> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| Skip)?;
    TcpStream::connect(listener.local_addr().unwrap()).map_err(|_| Skip)?;
    Ok(())
}

fn read_port(child: &mut Child) -> io::Result<u16> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing stdout"))?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        let res: io::Result<u16> = loop {
            match stdout.read(&mut byte) {
                Ok(0) => {
                    break Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "daemon closed",
                    ))
                }
                Ok(1) => {
                    if byte[0] == b'\n' {
                        break String::from_utf8(buf)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                            .and_then(|s| {
                                s.trim()
                                    .parse()
                                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                            });
                    }
                    buf.push(byte[0]);
                }
                Ok(_) => unreachable!(),
                Err(e) => break Err(e),
            }
        };
        let _ = tx.send(res);
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(res) => res,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            if child
                .wait_timeout(Duration::from_secs(0))
                .ok()
                .flatten()
                .is_some()
            {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "daemon exited before writing port",
                ))
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for daemon port",
                ))
            }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(io::Error::new(
            io::ErrorKind::Other,
            "failed to read daemon port",
        )),
    }
}

fn spawn_daemon() -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    Ok((child, port, dir))
}

fn spawn_temp_daemon() -> io::Result<(Child, u16, tempfile::TempDir)> {
    spawn_daemon()
}

#[test]
#[serial]
fn daemon_blocks_path_traversal() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, dir) = match spawn_temp_daemon() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    let parent = dir.path().parent().unwrap().to_path_buf();
    let secret = parent.join("secret");
    fs::write(&secret, b"top secret").unwrap();
    let dest = tempfile::tempdir().unwrap();
    let status = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            &format!("rsync://127.0.0.1:{port}/data/../secret"),
            dest.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!dest.path().join("secret").exists());
    let _ = child.kill();
    let _ = child.wait();
}

fn spawn_daemon_with_address(addr: &str) -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            "0",
            "--address",
            addr,
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    Ok((child, port, dir))
}

fn spawn_daemon_ipv4() -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            "0",
            "-4",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    Ok((child, port, dir))
}

fn spawn_daemon_ipv6() -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            "0",
            "-6",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    Ok((child, port, dir))
}

fn supports_ipv6() -> bool {
    TcpListener::bind(("::1", 0)).is_ok()
}

fn wait_for_daemon_v6(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("::1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
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
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
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
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon_with_address("127.0.0.1") {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    assert!(TcpStream::connect(("127.0.0.2", port)).is_err());
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_binds_with_ipv4_flag() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon_ipv4() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    assert!(TcpStream::connect(("::1", port)).is_err());
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_binds_with_ipv6_flag() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    if !supports_ipv6() {
        eprintln!("IPv6 unsupported; skipping test");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon_ipv6() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon_v6(port);
    TcpStream::connect(("::1", port)).unwrap();
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn probe_connects_to_daemon() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    Command::cargo_bin("oc-rsync")
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
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--probe", "--peer-version", "1"])
        .assert()
        .failure();
}

#[test]
#[serial]
fn daemon_accepts_connection_on_ephemeral_port() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_temp_daemon() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_allows_module_access() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_temp_daemon() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.authenticate(None, false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || !String::from_utf8_lossy(&buf[..n]).starts_with("@ERROR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_rejects_invalid_token() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
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
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("bad"), false).unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || String::from_utf8_lossy(&buf[..n]).starts_with("@ERR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_rejects_unauthorized_module() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
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
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("secret"), false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || String::from_utf8_lossy(&buf[..n]).starts_with("@ERR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_authenticates_valid_token() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
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
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("secret"), false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
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
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port) = {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let child = StdCommand::cargo_bin("oc-rsync")
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

    let (mut child, port) = {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let child = StdCommand::cargo_bin("oc-rsync")
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
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let motd = dir.path().join("motd");
    fs::write(&motd, "Hello world\n").unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    t.authenticate(None, false).unwrap();
    let mut motd_buf = [0u8; 64];
    let n = t.receive(&mut motd_buf).unwrap();
    assert!(String::from_utf8_lossy(&motd_buf[..n]).contains("Hello world"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_suppresses_motd_when_requested() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let motd = dir.path().join("motd");
    fs::write(&motd, "Hello world\n").unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    t.authenticate(None, true).unwrap();
    let mut motd_buf = [0u8; 64];
    let n = t.receive(&mut motd_buf).unwrap();
    assert_eq!(String::from_utf8_lossy(&motd_buf[..n]), "@RSYNCD: OK\n");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn client_respects_no_motd() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
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
    let mut child = StdCommand::cargo_bin("oc-rsync")
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

    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            &format!("rsync://127.0.0.1:{port}/data/"),
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("Hello world"));

    let output = Command::cargo_bin("oc-rsync")
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
fn daemon_writes_log_file() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("log");
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
        t.authenticate(None, false).unwrap();
        let mut ok = [0u8; 64];
        t.receive(&mut ok).unwrap();
        t.send(b"data\n").unwrap();
        t.send(b"\n").unwrap();
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
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let motd = dir.path().join("motd");
    let line = "A".repeat(256);
    fs::write(&motd, format!("{line}\nsecond")).unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("oc-rsync")
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
    t.authenticate(None, false).unwrap();
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
