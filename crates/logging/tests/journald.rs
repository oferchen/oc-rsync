// crates/logging/tests/journald.rs
#![cfg(all(unix, feature = "journald"))]

use logging::{DebugFlag, InfoFlag, LogFormat, SubscriberConfig, init};
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::info;

#[test]
fn journald_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    unsafe {
        std::env::set_var("OC_RSYNC_JOURNALD_PATH", &path);
    }
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(1)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(true)
        .colored(true)
        .timestamps(false)
        .build();
    init(cfg).unwrap();
    info!(target: "test", "hi");
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    let expected = "PRIORITY=6\nSYSLOG_IDENTIFIER=rsync\nMESSAGE=hi\n";
    assert_eq!(msg, expected);
    unsafe {
        std::env::remove_var("OC_RSYNC_JOURNALD_PATH");
    }
}
