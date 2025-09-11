// tests/daemon_config.rs

use assert_cmd::prelude::*;
use daemon::{Handler, handle_connection, load_config, parse_config};
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use transport::{LocalPipeTransport, TcpTransport, Transport};
mod common;
use common::{daemon::DaemonGuard, with_env_var};

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

fn spawn_daemon(config: &str) -> (DaemonGuard, u16, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, config).unwrap();
    let mut cmd = StdCommand::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = DaemonGuard::spawn(cmd);
    let port = read_port(&mut child);
    (child, port, dir)
}

struct MultiReader {
    parts: Vec<Vec<u8>>,
    idx: usize,
    pos: usize,
}

impl Read for MultiReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.idx >= self.parts.len() {
            return Ok(0);
        }
        let part = &self.parts[self.idx];
        let remaining = &part[self.pos..];
        let len = remaining.len().min(buf.len());
        buf[..len].copy_from_slice(&remaining[..len]);
        self.pos += len;
        if self.pos >= part.len() {
            self.idx += 1;
            self.pos = 0;
        }
        Ok(len)
    }
}

fn pipe_transport(token: &str, module: &str) -> LocalPipeTransport<MultiReader, Vec<u8>> {
    let parts = vec![
        LATEST_VERSION.to_be_bytes().to_vec(),
        format!("{token}\n").into_bytes(),
        format!("{module}\n\n").into_bytes(),
    ];
    let reader = MultiReader {
        parts,
        idx: 0,
        pos: 0,
    };
    LocalPipeTransport::new(reader, Vec::new())
}

#[test]
fn daemon_config_rsync_client() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("file.txt"), b"data").unwrap();
    let config = format!("port = 0\n[data]\n    path = {}\n", src.display());
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, &config).unwrap();
    let cfg = load_config(Some(&cfg_path)).unwrap();
    assert_eq!(cfg.modules[0].path, fs::canonicalize(&src).unwrap());
}

#[test]
fn daemon_config_authentication() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let cfg = format!(
        "port = 0\nuse chroot = no\nsecrets file = {}\n[data]\n    path = {}\n",
        secrets.display(),
        data.display()
    );
    let cfg = parse_config(&cfg).unwrap();
    let module = cfg.modules[0].clone();
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport("secret", "data");
    handle_connection(
        &mut t,
        &modules,
        Some(&secrets),
        None,
        None,
        None,
        None,
        true,
        &[],
        "127.0.0.1",
        0,
        0,
        &handler,
        None,
    )
    .unwrap();
    let (_, writer) = t.into_inner();
    let resp = String::from_utf8(writer).unwrap();
    assert!(resp.contains("@RSYNCD: OK"));
}

#[test]
fn daemon_config_motd_suppression() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let motd = dir.path().join("motd");
    fs::write(&motd, "Hello world\n").unwrap();

    let cfg = format!(
        "port = 0\nmotd file = {}\n[data]\n    path = {}\n",
        motd.display(),
        data.display()
    );
    let cfg = parse_config(&cfg).unwrap();
    let module = cfg.modules[0].clone();
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport("", "data");
    handle_connection(
        &mut t,
        &modules,
        None,
        None,
        None,
        None,
        cfg.motd_file.as_deref(),
        true,
        &[],
        "127.0.0.1",
        0,
        0,
        &handler,
        None,
    )
    .unwrap();
    let (_, writer) = t.into_inner();
    let resp = String::from_utf8(writer).unwrap();
    assert!(resp.contains("Hello world"));

    let cfg = format!(
        "port = 0\nmotd file =\n[data]\n    path = {}\n",
        data.display()
    );
    let cfg = parse_config(&cfg).unwrap();
    assert!(cfg.motd_file.is_none());
    let module = cfg.modules[0].clone();
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let mut t = pipe_transport("", "data");
    handle_connection(
        &mut t,
        &modules,
        None,
        None,
        None,
        None,
        cfg.motd_file.as_deref(),
        true,
        &[],
        "127.0.0.1",
        0,
        0,
        &handler,
        None,
    )
    .unwrap();
    let (_, writer) = t.into_inner();
    let resp = String::from_utf8(writer).unwrap();
    assert!(!resp.contains("Hello world"));
}

#[test]
#[serial]
fn daemon_config_host_filtering() {
    let allow_cfg = "port = 0\nhosts allow = 127.0.0.1\n[data]\n    path = /tmp\n";
    let (_daemon, port, _tmp) = spawn_daemon(allow_cfg);
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    let deny_cfg = "port = 0\nhosts deny = 127.0.0.1\n[data]\n    path = /tmp\n";
    let (_daemon, port, _tmp) = spawn_daemon(deny_cfg);
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let res = stream.read(&mut buf);
    assert!(res.is_err() || res.unwrap() == 0);
}

#[test]
fn daemon_config_module_secrets_file() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let cfg = format!(
        "port = 0\n[data]\n    path = {}\n    secrets file = {}\n",
        data.display(),
        secrets.display()
    );
    let cfg = parse_config(&cfg).unwrap();
    let module = cfg.modules[0].clone();
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport("secret", "data");
    handle_connection(
        &mut t,
        &modules,
        None,
        None,
        None,
        None,
        None,
        true,
        &[],
        "127.0.0.1",
        0,
        0,
        &handler,
        None,
    )
    .unwrap();
    let (_, writer) = t.into_inner();
    let resp = String::from_utf8(writer).unwrap();
    assert!(resp.contains("@RSYNCD: OK"));
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
    let mut cmd = StdCommand::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stderr(Stdio::null());
    let _guard = DaemonGuard::spawn(cmd);
    wait_for_daemon(port);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
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
    let (_child, port, _tmp) = spawn_daemon(&config);
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.authenticate(None, false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.receive(&mut ok).unwrap();
    t.send(b"--server\n").unwrap();
    t.send(b"\n").unwrap();
    let mut resp = [0u8; 128];
    let n = match t.receive(&mut resp) {
        Ok(n) => n,
        Err(err) => {
            assert_eq!(err.kind(), io::ErrorKind::TimedOut);
            0
        }
    };
    assert!(n == 0 || String::from_utf8_lossy(&resp[..n]).contains("read only"));
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
    let (_child, port, _tmp) = spawn_daemon(&config);
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
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
    let n = match t.receive(&mut resp) {
        Ok(n) => n,
        Err(err) => {
            assert_eq!(err.kind(), io::ErrorKind::TimedOut);
            0
        }
    };
    let msg = String::from_utf8_lossy(&resp[..n]);
    assert!(n == 0 || msg.contains("write only"));
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
#[serial]
fn load_config_default_path() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, "port = 873\n").unwrap();
    let var = "OC_RSYNC_CONFIG_PATH";
    with_env_var(var, &cfg_path, || {
        let cfg = load_config(None).unwrap();
        assert_eq!(cfg.port, Some(873));
    });
}

#[test]
fn load_config_custom_path() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, "port = 0\n").unwrap();
    let cfg = load_config(Some(&cfg_path)).unwrap();
    assert_eq!(cfg.port, Some(0));
}
