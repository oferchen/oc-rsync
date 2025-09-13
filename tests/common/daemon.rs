// tests/common/daemon.rs
#![allow(dead_code)]

use assert_cmd::cargo::{CommandCargoExt, cargo_bin};
#[cfg(unix)]
use nix::unistd::{Gid, Uid};
use std::fs;
use std::io::Read;
use std::net::TcpStream;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread::sleep;
use std::time::Duration;

pub struct DaemonGuard(Child);

impl DaemonGuard {
    pub fn spawn(mut cmd: StdCommand) -> Self {
        Self(cmd.spawn().unwrap())
    }
}

impl std::ops::Deref for DaemonGuard {
    type Target = Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for DaemonGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct Daemon {
    child: Child,
    pub port: u16,
}

impl std::ops::Deref for Daemon {
    type Target = Child;
    fn deref(&self) -> &Self::Target {
        &self.child
    }
}

impl std::ops::DerefMut for Daemon {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.child
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Daemon {
    pub fn new(child: Child, port: u16) -> Self {
        Daemon { child, port }
    }
}
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

pub fn spawn_daemon(root: &std::path::Path) -> Daemon {
    #[cfg(unix)]
    let (uid, gid) = (Uid::current().as_raw(), Gid::current().as_raw());
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--no-detach",
            "--module",
            &format!(
                "mod={},uid={},gid={},use-chroot=no,read-only=no",
                root.display(),
                uid,
                gid
            ),
            "--port",
            "0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
    Daemon { child, port }
}

pub fn spawn_rsync_daemon(root: &std::path::Path, extra: &str) -> Daemon {
    #[cfg(unix)]
    let (uid, gid) = (Uid::current().as_raw(), Gid::current().as_raw());
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);
    let conf = format!(
        "uid = {uid}\ngid = {gid}\nuse chroot = false\n[mod]\n  path = {}\n{}",
        root.display(),
        extra
    );
    let conf_path = root.join("rsyncd.conf");
    fs::write(&conf_path, conf).unwrap();
    let mut child = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            "0",
            "--config",
            conf_path.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);
    Daemon { child, port }
}

pub fn wait_for_daemon(daemon: &mut Daemon) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", daemon.port)).is_ok() {
            match daemon.child.try_wait() {
                Ok(None) => return,
                Ok(Some(status)) => panic!("daemon exited unexpectedly: {status}",),
                Err(e) => panic!("failed to query daemon status: {e}"),
            }
        }
        if let Ok(Some(status)) = daemon.child.try_wait() {
            panic!("daemon exited unexpectedly: {status}");
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}
