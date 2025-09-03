// crates/transport/tests/ssh_unknown_host.rs
#![allow(clippy::zombie_processes)]
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use tempfile::NamedTempFile;
use transport::ssh::SshStdioTransport;

#[test]
fn refuses_unknown_host_key() {
    if Command::new("/usr/sbin/sshd")
        .arg("-h")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("sshd not available; skipping test");
        return;
    }
    std::fs::create_dir_all("/run/sshd").expect("create /run/sshd");
    let mut sshd = Command::new("/usr/sbin/sshd")
        .arg("-D")
        .arg("-e")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sshd");

    thread::sleep(Duration::from_millis(500));

    let tmp = NamedTempFile::new().expect("tmp known_hosts");

    let transport = SshStdioTransport::spawn_server(
        "localhost",
        ["/"],
        &[],
        Some(tmp.path()),
        true,
        None,
        None,
    )
    .expect("spawn ssh");

    thread::sleep(Duration::from_millis(500));

    let (stderr, truncated) = transport.stderr();
    assert!(!truncated);
    let msg = String::from_utf8_lossy(&stderr);
    assert!(
        msg.contains("Host key verification failed"),
        "stderr: {msg}"
    );

    let _ = sshd.kill();
}
