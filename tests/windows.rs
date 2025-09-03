// tests/windows.rs
#![cfg(windows)]

use std::io::ErrorKind;
use transport::ssh::SshStdioTransport;

#[test]
fn ssh_transport_unavailable() {
    let err = SshStdioTransport::spawn("ssh", []).expect_err("ssh available");
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}
