// tests/daemon.rs
#![allow(clippy::io_other_error)]

use assert_cmd::prelude::*;
use assert_cmd::Command;
use daemon::{
    chroot_and_drop_privileges, drop_privileges, handle_connection, parse_config,
    parse_daemon_args, parse_module, Handler, Module,
};
use protocol::LATEST_VERSION;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[allow(unused_imports)]
use std::path::PathBuf;
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::{mpsc, Arc};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use transport::{AddressFamily, LocalPipeTransport, TcpTransport, Transport};
use wait_timeout::ChildExt;

struct Skip;

fn require_network() -> Result<(), Skip> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| Skip)?;
    TcpStream::connect(listener.local_addr().unwrap()).map_err(|_| Skip)?;
    Ok(())
}

#[test]
fn parse_daemon_args_parses_options() {
    let args = vec![
        "--address".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        "1234".to_string(),
        "--ipv4".to_string(),
    ];
    let opts = parse_daemon_args(args).unwrap();
    assert_eq!(opts.address, Some("127.0.0.1".parse().unwrap()));
    assert_eq!(opts.port, 1234);
    assert!(matches!(opts.family, Some(AddressFamily::V4)));
}

#[test]
fn parse_daemon_args_rejects_mismatch() {
    let args = vec![
        "--address".to_string(),
        "127.0.0.1".to_string(),
        "--ipv6".to_string(),
    ];
    assert!(parse_daemon_args(args).is_err());
}

#[test]
fn parse_daemon_args_invalid_port() {
    let args = vec!["--port".to_string(), "not-a-number".to_string()];
    assert!(parse_daemon_args(args).is_err());
}

#[test]
fn parse_module_parses_options() {
    let dir = tempfile::tempdir().unwrap();
    let auth = dir.path().join("auth");
    fs::write(&auth, "alice data\n").unwrap();
    let spec = format!(
        "data={},hosts-allow=127.0.0.1,hosts-deny=10.0.0.1,auth-users=alice bob,secrets-file={},uid=0,gid=0,timeout=1,use-chroot=no,numeric-ids=yes",
        dir.path().display(),
        auth.display()
    );
    let module = parse_module(&spec).unwrap();
    assert_eq!(module.name, "data");
    assert_eq!(module.path, fs::canonicalize(dir.path()).unwrap());
    assert_eq!(module.hosts_allow, vec!["127.0.0.1".to_string()]);
    assert_eq!(module.hosts_deny, vec!["10.0.0.1".to_string()]);
    assert_eq!(
        module.auth_users,
        vec!["alice".to_string(), "bob".to_string()]
    );
    assert_eq!(module.secrets_file, Some(PathBuf::from(&auth)));
    assert_eq!(module.uid, Some(0));
    assert_eq!(module.gid, Some(0));
    assert_eq!(module.timeout, Some(Duration::from_secs(1)));
    assert!(!module.use_chroot);
    assert!(module.numeric_ids);
}

#[cfg(unix)]
#[test]
fn parse_module_resolves_named_uid_gid() {
    let spec = "data=/tmp,uid=root,gid=root";
    let module = parse_module(spec).unwrap();
    assert_eq!(module.uid, Some(0));
    assert_eq!(module.gid, Some(0));
}

#[test]
fn parse_config_accepts_hyphenated_keys() {
    let cfg = parse_config("motd-file=/tmp/m\n[data]\npath=/tmp\n").unwrap();
    assert_eq!(cfg.motd_file, Some(PathBuf::from("/tmp/m")));
}

#[test]
fn parse_config_requires_module_path() {
    assert!(parse_config("[data]\nuid=0\n").is_err());
}

#[cfg(unix)]
#[test]
fn parse_config_resolves_symlink_path() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let link = dir.path().join("link");
    symlink(dir.path(), &link).unwrap();
    let cfg = parse_config(&format!("[data]\npath={}\n", link.display())).unwrap();
    assert_eq!(cfg.modules[0].path, fs::canonicalize(&link).unwrap());
}

#[cfg(unix)]
#[test]
#[serial]
fn chroot_drops_privileges() {
    use nix::sys::wait::waitpid;
    use nix::unistd::{fork, getegid, geteuid, ForkResult};

    let dir = tempdir().unwrap();
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let status = waitpid(child, None).unwrap();
            assert!(matches!(status, nix::sys::wait::WaitStatus::Exited(_, 0)));
        }
        Ok(ForkResult::Child) => {
            chroot_and_drop_privileges(dir.path(), 1, 1, true).unwrap();
            assert_eq!(std::env::current_dir().unwrap(), PathBuf::from("/"));
            assert_eq!(geteuid().as_raw(), 1);
            assert_eq!(getegid().as_raw(), 1);
            std::process::exit(0);
        }
        Err(_) => panic!("fork failed"),
    }
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
    let writer = Vec::new();
    LocalPipeTransport::new(reader, writer)
}

#[test]
#[serial]
fn module_authentication_and_hosts_enforced() {
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
    let handler: Arc<Handler> = Arc::new(|_| Ok(()));
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
        0,
        0,
        &handler,
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
        0,
        0,
        &handler,
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
        0,
        0,
        &handler,
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
    let handler: Arc<Handler> = Arc::new(|_| Ok(()));
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
    let handler: Arc<Handler> = Arc::new(|_| Ok(()));
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
    )
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[cfg(unix)]
#[test]
#[serial]
fn drop_privileges_changes_ids() {
    use nix::sys::wait::waitpid;
    use nix::unistd::{fork, getegid, geteuid, ForkResult};
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let status = waitpid(child, None).unwrap();
            assert!(matches!(status, nix::sys::wait::WaitStatus::Exited(_, 0)));
        }
        Ok(ForkResult::Child) => {
            drop_privileges(1, 1).unwrap();
            assert_eq!(geteuid().as_raw(), 1);
            assert_eq!(getegid().as_raw(), 1);
            std::process::exit(0);
        }
        Err(_) => panic!("fork failed"),
    }
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
        .stderr(Stdio::null())
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

fn spawn_daemon_with_timeout(timeout: u64) -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            "0",
            "--timeout",
            &timeout.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
    let output = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
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

#[test]
#[serial]
#[ignore]
fn daemon_allows_path_traversal_without_chroot() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secret = dir.path().join("secret");
    fs::write(&secret, b"top secret").unwrap();
    let cfg = format!(
        "port = 0\n[data]\n    path = {}\n    use chroot = no\n",
        data.display()
    );
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, cfg).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    let dest = tempfile::tempdir().unwrap();
    let output = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            &format!("rsync://127.0.0.1:{port}/data/../secret"),
            dest.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(dest.path().join("secret").exists());
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_drops_privileges_and_restricts_file_access() {
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
    let secret = dir.path().join("secret");
    fs::write(&secret, b"top secret").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secret, fs::Permissions::from_mode(0o600)).unwrap();
    let dest = tempfile::tempdir().unwrap();
    let output = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
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

#[test]
#[serial]
fn daemon_enforces_timeout() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon_with_timeout(1) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
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
        .stderr(Stdio::null())
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

fn spawn_daemon_with_config_address(addr: &str) -> io::Result<(Child, u16, tempfile::TempDir)> {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let cfg = format!(
        "port = 0\naddress = {addr}\n[data]\n    path = {}\n",
        data.display()
    );
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, cfg).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
        .stderr(Stdio::null())
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
        .stderr(Stdio::null())
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
            sleep(Duration::from_millis(50));
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
fn daemon_config_binds_to_specified_address() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let (mut child, port, _dir) = match spawn_daemon_with_config_address("127.0.0.1") {
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
fn daemon_runs_with_numeric_ids() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", dir.path().display()),
            "--port",
            "0",
            "--numeric-ids",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
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
#[ignore]
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
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    t.authenticate(None, false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_rejects_world_readable_secrets_file() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o644)).unwrap();
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("secret"), false).unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(Some("bad"), false).unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_rejects_missing_token() {
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    t.authenticate(None, false).unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
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
fn daemon_accepts_authorized_client() {
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
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
fn daemon_accepts_listed_auth_user() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "alice data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let cfg = format!(
        "port = 0\nsecrets file = {}\n[data]\n    path = {}\n    auth users = alice\n",
        secrets.display(),
        data.display()
    );
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, cfg).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    t.authenticate(Some("alice"), false).unwrap();
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
fn daemon_rejects_unlisted_auth_user() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    fs::create_dir(&data).unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "alice data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let cfg = format!(
        "port = 0\nsecrets file = {}\n[data]\n    path = {}\n    auth users = alice\n",
        secrets.display(),
        data.display()
    );
    let cfg_path = dir.path().join("rsyncd.conf");
    fs::write(&cfg_path, cfg).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--daemon", "--config", cfg_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = match read_port(&mut child) {
        Ok(p) => p,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            eprintln!("skipping daemon test: {e}");
            return;
        }
    };
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    t.authenticate(Some("bob"), false).unwrap();
    let mut resp = [0u8; 64];
    let n = t.receive(&mut resp).unwrap_or(0);
    let msg = String::from_utf8_lossy(&resp[..n]);
    assert!(n == 0 || msg.starts_with("@ERROR"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
#[ignore]
fn daemon_parses_secrets_file_with_comments() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "# comment\n\nsecret data\n").unwrap();
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);
    t.authenticate(Some("secret"), false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    let _ = t.receive(&mut buf).unwrap_or(0);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
#[ignore]
fn client_authenticates_with_password_file() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("auth");
    fs::write(&secrets, "secret data\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&secrets, fs::Permissions::from_mode(0o600)).unwrap();
    let pw = dir.path().join("pw");
    fs::write(&pw, "secret\n").unwrap();
    #[cfg(unix)]
    fs::set_permissions(&pw, fs::Permissions::from_mode(0o600)).unwrap();
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
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut t = TcpTransport::connect("127.0.0.1", port, None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    let token = fs::read_to_string(&pw).unwrap();
    t.authenticate(Some(token.trim()), false).unwrap();
    let mut ok = [0u8; 64];
    t.receive(&mut ok).unwrap();
    t.send(b"data\n").unwrap();
    t.send(b"\n").unwrap();
    let n = t.receive(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
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
            .stderr(Stdio::null())
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
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        (child, port)
    };
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    let res = stream.read(&mut buf).unwrap();
    assert!(
        res == 0
            || std::str::from_utf8(&buf[..res])
                .unwrap()
                .starts_with("@ERROR")
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
fn daemon_respects_module_host_lists() {
    if require_network().is_err() {
        eprintln!("skipping daemon test: network access required");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("rsyncd.conf");

    fs::write(&cfg, "[data]\npath=/tmp\nhosts allow=127.0.0.1\n").unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--config",
            cfg.to_str().unwrap(),
            "--port",
            &port.to_string(),
        ])
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    let _ = child.kill();
    let _ = child.wait();

    fs::write(&cfg, "[data]\npath=/tmp\nhosts deny=127.0.0.1\n").unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--config",
            cfg.to_str().unwrap(),
            "--port",
            &port.to_string(),
        ])
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_daemon(port);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 256];
    stream.read_exact(&mut buf[..4]).unwrap();
    stream.write_all(b"data\n").unwrap();
    let res = stream.read(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..res]).unwrap();
    assert!(res == 0 || msg.starts_with("@ERROR") || msg.starts_with("@RSYNCD"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
#[serial]
#[ignore]
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
        .stderr(Stdio::null())
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
        .stderr(Stdio::null())
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
#[ignore]
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
        .stderr(Stdio::null())
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
        .stderr(Stdio::null())
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
#[ignore]
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
        .stderr(Stdio::null())
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
    assert!(start.elapsed() >= Duration::from_millis(400));
    let _ = child.kill();
    let _ = child.wait();
}
