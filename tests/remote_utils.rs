// tests/remote_utils.rs
#![cfg(unix)]
use transport::ssh::SshStdioTransport;

pub fn spawn_reader(cmd: &str) -> SshStdioTransport {
    SshStdioTransport::spawn("sh", ["-c", cmd]).expect("spawn")
}

pub fn spawn_writer(cmd: &str) -> SshStdioTransport {
    SshStdioTransport::spawn("sh", ["-c", cmd]).expect("spawn")
}
