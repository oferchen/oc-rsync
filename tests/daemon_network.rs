// tests/daemon_network.rs
#![cfg(feature = "network")]

use assert_cmd::cargo::cargo_bin;
use daemon::{Handler, Module, handle_connection, host_allowed};
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Cursor, Read};
use std::net::{IpAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;
use transport::LocalPipeTransport;
#[cfg(feature = "network")]
use transport::TcpTransport;
use wait_timeout::ChildExt;

#[cfg(feature = "network")]
mod common;
#[cfg(feature = "network")]
use common::oc_cmd;

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
    let writer = Vec::new();
    LocalPipeTransport::new(reader, writer)
}

fn pipe_transport_opts(
    token: &str,
    module: &str,
    opts: &[&str],
) -> LocalPipeTransport<MultiReader, Vec<u8>> {
    let mut parts = vec![
        LATEST_VERSION.to_be_bytes().to_vec(),
        format!("{token}\n").into_bytes(),
        format!("{module}\n").into_bytes(),
    ];
    for opt in opts {
        parts.push(format!("{opt}\n").into_bytes());
    }
    parts.push(b"\n".to_vec());
    let reader = MultiReader {
        parts,
        idx: 0,
        pos: 0,
    };
    let writer = Vec::new();
    LocalPipeTransport::new(reader, writer)
}

#[test]
#[serial]
fn module_authentication_and_hosts_enforced() {
    use nix::unistd::{getegid, geteuid};
    let uid = geteuid().as_raw();
    let gid = getegid().as_raw();
    let dir = tempdir().unwrap();
    let auth = dir.path().join("auth");
    fs::write(&auth, "alice data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&auth, fs::Permissions::from_mode(0o600)).unwrap();
    let module = Module {
        name: "data".to_string(),
        path: std::env::current_dir().unwrap(),
        auth_users: vec!["alice".to_string()],
        hosts_allow: vec!["127.0.0.1".to_string()],
        use_chroot: false,
        ..Module::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut ok_t = pipe_transport("alice", "data");
    handle_connection(
        &mut ok_t,
        &modules,
        Some(&auth),
        None,
        None,
        None,
        None,
        true,
        &[],
        "127.0.0.1",
        uid,
        gid,
        &handler,
        None,
    )
    .unwrap();

    let mut bad_user = pipe_transport("bob", "data");
    let err = handle_connection(
        &mut bad_user,
        &modules,
        Some(&auth),
        None,
        None,
        None,
        None,
        true,
        &[],
        "127.0.0.1",
        uid,
        gid,
        &handler,
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    let mut bad_host = pipe_transport("", "data");
    let err = handle_connection(
        &mut bad_host,
        &modules,
        None,
        None,
        None,
        None,
        None,
        true,
        &[],
        "10.0.0.1",
        uid,
        gid,
        &handler,
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn host_deny_blocks_connection() {
    let module = Module {
        name: "data".into(),
        path: std::env::current_dir().unwrap(),
        hosts_deny: vec!["127.0.0.1".into()],
        use_chroot: false,
        ..Module::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport("", "data");
    let err = handle_connection(
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
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
#[serial]
fn host_allow_supports_cidr() {
    let ip: IpAddr = "127.0.0.1".parse().unwrap();
    assert!(host_allowed(&ip, &["127.0.0.0/8".into()], &[]));
    assert!(!host_allowed(&ip, &[], &["127.0.0.0/24".into()]));
}

#[test]
#[serial]
fn daemon_refuses_configured_option() {
    let module = Module {
        name: "data".into(),
        path: std::env::current_dir().unwrap(),
        refuse_options: vec!["--delete".into()],
        use_chroot: false,
        ..Module::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport_opts("", "data", &["--delete"]);
    let err = handle_connection(
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
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
#[serial]
fn daemon_refuses_numeric_ids_option() {
    let module = Module {
        name: "data".into(),
        path: std::env::current_dir().unwrap(),
        use_chroot: false,
        ..Module::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport_opts("", "data", &["--numeric-ids"]);
    let err = handle_connection(
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
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
#[serial]
fn daemon_refuses_no_numeric_ids_option() {
    let module = Module {
        name: "data".into(),
        path: std::env::current_dir().unwrap(),
        numeric_ids: true,
        use_chroot: false,
        ..Module::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport_opts("", "data", &["--no-numeric-ids"]);
    let err = handle_connection(
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
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn rejects_missing_token() {
    let dir = tempdir().unwrap();
    let auth = dir.path().join("auth");
    fs::write(&auth, "tok data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&auth, fs::Permissions::from_mode(0o600)).unwrap();
    let module = Module {
        name: "data".into(),
        path: std::env::current_dir().unwrap(),
        secrets_file: Some(auth.clone()),
        use_chroot: false,
        ..Module::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let mut t = pipe_transport("", "data");
    let err = handle_connection(
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
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn anonymous_module_listing_only_shows_listed_modules() {
    let mut input = Vec::new();
    input.extend_from_slice(&LATEST_VERSION.to_be_bytes());
    input.extend_from_slice(b"\n\n");
    let reader = Cursor::new(input);
    let writer = Cursor::new(Vec::new());
    let mut transport = LocalPipeTransport::new(reader, writer);

    let mut modules = HashMap::new();
    let public = Module {
        name: "public".into(),
        list: true,
        ..Module::default()
    };
    modules.insert("public".into(), public);
    let private = Module {
        name: "private".into(),
        list: false,
        ..Module::default()
    };
    modules.insert("private".into(), private);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));

    handle_connection(
        &mut transport,
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

    let (_, writer) = transport.into_inner();
    let out = writer.into_inner();
    let text = String::from_utf8_lossy(&out[4..]);
    assert!(text.contains("public"));
    assert!(!text.contains("private"));
}

fn read_port(child: &mut Child) -> io::Result<u16> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout"))?;
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
                    ));
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
                Err(io::Error::other("daemon exited before writing port"))
            } else {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for daemon port",
                ))
            }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(io::Error::other("failed to read daemon port"))
        }
    }
}

fn wait_for_daemon(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            sleep(Duration::from_millis(50));
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}

fn spawn_daemon() -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let module_path = fs::canonicalize(dir.path()).unwrap();
    let program = cargo_bin("oc-rsync");
    let mut cmd = Command::new(program);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.args([
        "--daemon",
        "--no-detach",
        "--module",
        &format!("data={}", module_path.display()),
        "--port",
        "0",
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    let mut child = cmd.spawn().unwrap();
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

fn spawn_daemon_with_timeout(timeout: u64) -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let module_path = fs::canonicalize(dir.path()).unwrap();
    let program = cargo_bin("oc-rsync");
    let mut cmd = Command::new(program);
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd.args([
        "--daemon",
        "--no-detach",
        "--module",
        &format!("data={}", module_path.display()),
        "--port",
        "0",
        "--timeout",
        &timeout.to_string(),
    ])
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    let mut child = cmd.spawn().unwrap();
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

#[cfg(feature = "network")]
#[test]
#[serial]
#[ignore = "requires network access"]
fn daemon_blocks_path_traversal() {
    let (mut child, port, dir) = spawn_temp_daemon().expect("spawn daemon");
    wait_for_daemon(port);
    let parent = dir.path().parent().unwrap().to_path_buf();
    let secret = parent.join("secret");
    fs::write(&secret, b"top secret").unwrap();
    let dest = tempfile::tempdir().unwrap();
    let output = oc_cmd()
        .args([
            &format!("rsync://127.0.0.1:{port}/data/../secret"),
            dest.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dest.path().join("secret").exists());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(feature = "network")]
#[test]
#[serial]
#[ignore = "requires network access"]
fn daemon_drops_privileges_and_restricts_file_access() {
    let (mut child, port, dir) = spawn_temp_daemon().expect("spawn daemon");
    wait_for_daemon(port);
    let secret = dir.path().join("secret");
    fs::write(&secret, b"top secret").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secret, fs::Permissions::from_mode(0o600)).unwrap();
    let dest = tempfile::tempdir().unwrap();
    let output = oc_cmd()
        .args([
            &format!("rsync://127.0.0.1:{port}/data/secret"),
            dest.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dest.path().join("secret").exists());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(feature = "network")]
#[test]
#[serial]
#[ignore = "requires network access"]
fn daemon_enforces_timeout() {
    let (mut child, port, _dir) = spawn_daemon_with_timeout(1).expect("spawn daemon");
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    sleep(Duration::from_secs(2));
    let mut buf = [0u8; 1];
    match t.receive(&mut buf) {
        Err(e) => assert!(
            e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::ConnectionReset
        ),
        Ok(0) => (),
        Ok(_) => panic!("unexpected data"),
    }
    let _ = child.kill();
    let _ = child.wait();
}
