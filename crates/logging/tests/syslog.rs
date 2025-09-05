// crates/logging/tests/syslog.rs
#![cfg(all(unix, feature = "syslog"))]

use logging::{init, DebugFlag, InfoFlag, LogFormat, SubscriberConfig};
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::info;

#[test]
fn syslog_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    std::env::set_var("OC_RSYNC_SYSLOG_PATH", &path);
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(1)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(true)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    init(cfg);
    info!(target: "test", "hello");
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    let expected = format!("<14>rsync[{}]: hello", std::process::id());
    assert_eq!(msg, expected);
    std::env::remove_var("OC_RSYNC_SYSLOG_PATH");
}
