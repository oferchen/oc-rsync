// tests/daemon_syslog.rs
#![cfg(all(unix, not(feature = "nightly")))]

use daemon::init_logging;
use serial_test::serial;
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::warn;

mod common;
use common::temp_env;

#[test]
#[serial]
fn daemon_syslog_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    let _env = temp_env("OC_RSYNC_SYSLOG_PATH", &path);
    init_logging(None, None, true, false, false).unwrap();
    warn!(target: "test", "daemon syslog");
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    let expected = format!("<12>rsync[{}]: daemon syslog", std::process::id());
    assert_eq!(msg, expected);
}
