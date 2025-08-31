// tests/remote_utils.rs

use transport::ssh::SshStdioTransport;

pub fn spawn_reader(cmd: &str) -> SshStdioTransport {
    SshStdioTransport::spawn("sh", ["-c", cmd]).expect("spawn")
}

pub fn spawn_writer(cmd: &str) -> SshStdioTransport {
    SshStdioTransport::spawn("sh", ["-c", cmd]).expect("spawn")
}
