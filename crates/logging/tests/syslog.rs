// crates/logging/tests/syslog.rs
#![cfg(all(unix, feature = "syslog"))]

use logging::{subscriber, LogFormat};
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::info;
use tracing::subscriber::with_default;

#[test]
fn syslog_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    std::env::set_var("OC_RSYNC_SYSLOG_PATH", &path);
    let sub = subscriber(LogFormat::Text, 0, &[], &[], false, None, true, false);
    with_default(sub, || {
        info!(target: "test", "hello");
    });
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    assert!(msg.contains("hello"));
    std::env::remove_var("OC_RSYNC_SYSLOG_PATH");
}
