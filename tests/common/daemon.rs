// tests/common/daemon.rs
#![allow(dead_code)]

use assert_cmd::cargo::{CommandCargoExt, cargo_bin};
#[cfg(unix)]
use nix::unistd::{Gid, Uid};
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command as StdCommand};
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

pub fn spawn_daemon(root: &std::path::Path) -> Daemon {
    #[cfg(unix)]
    let (uid, gid) = (Uid::current().as_raw(), Gid::current().as_raw());
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--no-detach",
            "--module",
            &format!(
                "mod={},uid={},gid={},use-chroot=no",
                root.display(),
                uid,
                gid
            ),
            "--port",
            &port.to_string(),
        ])
        .spawn()
        .unwrap();
    Daemon { child, port }
}

pub fn spawn_rsync_daemon(root: &std::path::Path, extra: &str) -> Daemon {
    #[cfg(unix)]
    let (uid, gid) = (Uid::current().as_raw(), Gid::current().as_raw());
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let conf = format!(
        "uid = {uid}\ngid = {gid}\nuse chroot = false\n[mod]\n  path = {}\n{}",
        root.display(),
        extra
    );
    let conf_path = root.join("rsyncd.conf");
    fs::write(&conf_path, conf).unwrap();
    let child = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            &port.to_string(),
            "--config",
            conf_path.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    Daemon { child, port }
}

pub fn wait_for_daemon(port: u16) {
    for _ in 0..20 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}
