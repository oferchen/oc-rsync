// tests/daemon_config.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread::sleep;
use std::time::Duration;
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

fn wait_for_daemon(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}

fn spawn_daemon(config: &str) -> (Child, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, config).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
    (child, port, dir)
}

#[test]
#[serial]
fn daemon_config_authentication() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let config = format!(
        "port = 0\nsecrets file = {}\n[data]\n    path = {}\n",
        secrets.display(),
        data.display()
    );
    let (mut child, port, _tmp) = spawn_daemon(&config);
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.authenticate(Some("secret")).unwrap();
    t.send(b"data\n").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || !String::from_utf8_lossy(&buf[..n]).starts_with("@ERROR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_config_motd_suppression() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let dst = dir.path().join("dst");
    fs::create_dir(&dst).unwrap();
    let motd = dir.path().join("motd");
    fs::write(&motd, "Hello world\n").unwrap();
    let config = format!(
        "port = 0\nmotd file = {}\n[data]\n    path = {}\n",
        motd.display(),
        src.display()
    );
    let (mut child, port, _tmp) = spawn_daemon(&config);
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
fn daemon_config_host_filtering() {
    let allow_cfg = "port = 0\nhosts allow = 127.0.0.1\n[data]\n    path = /tmp\n";
    let (mut child, port, _tmp) = spawn_daemon(allow_cfg);
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    let _ = child.kill();
    let _ = child.wait();

    let deny_cfg = "port = 0\nhosts deny = 127.0.0.1\n[data]\n    path = /tmp\n";
    let (mut child, port, _tmp) = spawn_daemon(deny_cfg);
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let res = stream.read(&mut buf);
    assert!(res.is_err() || res.unwrap() == 0);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_config_module_secrets_file() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let config = format!(
        "port = 0\n[data]\n    path = {}\n    secrets file = {}\n",
        data.display(),
        secrets.display()
    );
    let (mut child, port, _tmp) = spawn_daemon(&config);
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.authenticate(Some("secret")).unwrap();
    t.send(b"data\n").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert!(n == 0 || !String::from_utf8_lossy(&buf[..n]).starts_with("@ERROR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_config_custom_port() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let cfg = format!("port = {port}\n[data]\n    path = {}\n", data.display());
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, cfg).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    let _ = child.kill();
    let _ = child.wait();
}
