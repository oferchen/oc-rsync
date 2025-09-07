// tests/daemon_journald.rs
#![cfg(all(unix, not(feature = "nightly")))]

use daemon::init_logging;
use serial_test::serial;
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::warn;

#[test]
#[serial]
fn daemon_journald_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    unsafe { std::env::set_var("OC_RSYNC_JOURNALD_PATH", &path) };
    init_logging(None, None, false, true, false).unwrap();
    warn!(target: "test", "daemon journald");
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    let expected = "PRIORITY=4\nSYSLOG_IDENTIFIER=rsync\nMESSAGE=daemon journald\n";
    assert_eq!(msg, expected);
    unsafe { std::env::remove_var("OC_RSYNC_JOURNALD_PATH") };
}
