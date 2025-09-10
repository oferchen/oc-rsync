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

pub fn spawn_daemon(root: &std::path::Path) -> (Child, u16) {
    #[cfg(unix)]
    let (uid, gid) = (Uid::current().as_raw(), Gid::current().as_raw());
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);
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
    (child, port)
}

pub fn spawn_rsync_daemon(root: &std::path::Path, extra: &str) -> (Child, u16) {
    #[cfg(unix)]
    let (uid, gid) = (Uid::current().as_raw(), Gid::current().as_raw());
    #[cfg(not(unix))]
    let (uid, gid) = (0, 0);
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
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
    (child, port)
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
