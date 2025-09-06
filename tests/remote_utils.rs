// tests/remote_utils.rs

use std::process::Command;
use transport::ssh::SshStdioTransport;

pub fn spawn_reader(cmd: &str) -> SshStdioTransport {
    let mut c = Command::new("sh");
    c.args(["-c", cmd])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .env("TZ", "UTC");
    SshStdioTransport::spawn_from_command(c, false).expect("spawn")
}

pub fn spawn_writer(cmd: &str) -> SshStdioTransport {
    let mut c = Command::new("sh");
    c.args(["-c", cmd])
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .env("TZ", "UTC");
    SshStdioTransport::spawn_from_command(c, false).expect("spawn")
}
