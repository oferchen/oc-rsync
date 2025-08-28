use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use tempfile::NamedTempFile;
use transport::ssh::SshStdioTransport;

#[test]
fn refuses_unknown_host_key() {
    // Start a local SSH server in the background.
    std::fs::create_dir_all("/run/sshd").expect("create /run/sshd");
    let mut sshd = Command::new("/usr/sbin/sshd")
        .arg("-D")
        .arg("-e")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sshd");

    // Give the server a moment to start listening on port 22.
    thread::sleep(Duration::from_millis(500));

    // Use an empty known_hosts file to ensure the host key is unknown.
    let tmp = NamedTempFile::new().expect("tmp known_hosts");

    let transport = SshStdioTransport::spawn_server(
        "localhost",
        ["/"],
        Some(tmp.path()),
        true,
    )
    .expect("spawn ssh");

    // Give the ssh process time to emit its failure message.
    thread::sleep(Duration::from_millis(500));

    let stderr = transport.stderr();
    let msg = String::from_utf8_lossy(&stderr);
    assert!(msg.contains("Host key verification failed"), "stderr: {msg}");

    let _ = sshd.kill();
}


