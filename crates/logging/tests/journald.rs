// crates/logging/tests/journald.rs
use logging::{subscriber, LogFormat};
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::info;
use tracing::subscriber::with_default;

#[test]
fn journald_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    std::env::set_var("OC_RSYNC_JOURNALD_PATH", &path);
    let sub = subscriber(LogFormat::Text, 0, &[], &[], false, None, false, true);
    with_default(sub, || {
        info!(target: "test", "hi");
    });
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    assert!(msg.contains("MESSAGE=hi"));
    std::env::remove_var("OC_RSYNC_JOURNALD_PATH");
}
