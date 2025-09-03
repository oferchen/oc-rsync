// tests/daemon_config.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use daemon::{load_config, parse_config};
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
    (child, port, dir)
}

#[test]
#[serial]
#[ignore]
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
    t.authenticate(Some("secret"), false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
#[ignore]
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
#[ignore]
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
    t.authenticate(Some("secret"), false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_config_read_only_module_rejects_writes() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    fs::create_dir(&data_dir).unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("file.txt"), b"data").unwrap();
    let config = format!(
        "port = 0\n[data]\n    path = {}\n    read only = yes\n",
        data_dir.display()
    );
    let (mut child, port, _tmp) = spawn_daemon(&config);
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.send(&0u32.to_be_bytes()).unwrap();
    t.receive(&mut buf).unwrap();
    t.authenticate(None, false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.receive(&mut ok).unwrap();
    t.send(b"--server\n").unwrap();
    t.send(b"\n").unwrap();
    let mut resp = [0u8; 128];
    let n = t.receive(&mut resp).unwrap_or(0);
    assert!(String::from_utf8_lossy(&resp[..n]).contains("read only"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_config_write_only_module_rejects_reads() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("data");
    fs::create_dir(&data_dir).unwrap();
    let config = format!(
        "port = 0\n[data]\n    path = {}\n    write only = yes\n",
        data_dir.display()
    );
    let (mut child, port, _tmp) = spawn_daemon(&config);
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.authenticate(None, false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.receive(&mut ok).unwrap();
    t.send(b"--server\n--sender\n").unwrap();
    t.send(b"\n").unwrap();
    let mut resp = [0u8; 128];
    let n = t.receive(&mut resp).unwrap_or(0);
    assert!(n == 0 || String::from_utf8_lossy(&resp[..n]).contains("write only"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn parse_config_global_directives() {
    let cfg = parse_config(
        "read only = yes\nlist = no\nmax connections = 5\nrefuse options = delete, compress\n[data]\n    path = /tmp\n",
    )
    .unwrap();
    assert_eq!(cfg.read_only, Some(true));
    assert_eq!(cfg.list, Some(false));
    assert_eq!(cfg.max_connections, Some(5));
    assert_eq!(
        cfg.refuse_options,
        vec!["delete".to_string(), "compress".to_string()]
    );
}

#[test]
fn parse_config_global_write_only() {
    let cfg = parse_config("write only = yes\n[data]\n    path = /tmp\n").unwrap();
    assert_eq!(cfg.write_only, Some(true));
}

#[test]
fn parse_config_module_comment_and_write_only() {
    let cfg = parse_config("[data]\npath=/tmp\ncomment = test\nwrite only = yes\n").unwrap();
    assert_eq!(cfg.modules[0].comment.as_deref(), Some("test"));
    assert!(cfg.modules[0].write_only);
}

#[test]
fn parse_config_inline_comments_and_modules() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();
    let cfg = format!(
        "port = 0 # global comment\n[first] ; module one\n    path = {} # path comment\n[second]\n    path = {}\n",
        dir1.path().display(),
        dir2.path().display()
    );
    let cfg = parse_config(&cfg).unwrap();
    assert_eq!(cfg.port, Some(0));
    assert_eq!(cfg.modules.len(), 2);
    assert_eq!(cfg.modules[0].name, "first");
    assert_eq!(cfg.modules[1].name, "second");
}

#[test]
fn parse_config_value_contains_hash() {
    let cfg = parse_config("log file = /tmp/rsync#log\n[data]\n    path = /tmp\n").unwrap();
    assert_eq!(cfg.log_file, Some(PathBuf::from("/tmp/rsync#log")));
}

#[test]
fn parse_config_value_contains_semicolon() {
    let cfg = parse_config("log file = /tmp/rsync;log\n[data]\n    path = /tmp\n").unwrap();
    assert_eq!(cfg.log_file, Some(PathBuf::from("/tmp/rsync;log")));
}

#[test]
fn parse_config_value_contains_comment_chars_inside_quotes() {
    let cfg =
        parse_config("log file = \"/tmp/rsync #log ;semi\"\n[data]\n    path = /tmp\n").unwrap();
    assert_eq!(
        cfg.log_file,
        Some(PathBuf::from("\"/tmp/rsync #log ;semi\""))
    );
}

#[test]
fn load_config_default_path() {
    let path = Path::new("/etc/oc-rsyncd.conf");
    let backup = fs::read_to_string(path).ok();
    fs::write(path, "port = 873\n").unwrap();
    let cfg = load_config(None).unwrap();
    assert_eq!(cfg.port, Some(873));
    if let Some(contents) = backup {
        fs::write(path, contents).unwrap();
    } else {
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn load_config_custom_path() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, "port = 0\n").unwrap();
    let cfg = load_config(Some(&cfg_path)).unwrap();
    assert_eq!(cfg.port, Some(0));
}
