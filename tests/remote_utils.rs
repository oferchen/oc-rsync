#![cfg(unix)]
use transport::ssh::SshStdioTransport;

/// Spawn a "remote" reader using the local shell.
pub fn spawn_reader(cmd: &str) -> SshStdioTransport {
    SshStdioTransport::spawn("sh", ["-c", cmd]).expect("spawn")
}

/// Spawn a "remote" writer using the local shell.
pub fn spawn_writer(cmd: &str) -> SshStdioTransport {
    SshStdioTransport::spawn("sh", ["-c", cmd]).expect("spawn")
}
