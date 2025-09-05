// tests/daemon_syslog.rs
#![cfg(unix)]

use daemon::init_logging;
use serial_test::serial;
use std::ffi::OsStr;
use std::os::unix::net::UnixDatagram;
use tempfile::tempdir;
use tracing::warn;

fn set_env_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, val: V) {
    unsafe { std::env::set_var(key, val) }
}

fn remove_env_var<K: AsRef<OsStr>>(key: K) {
    unsafe { std::env::remove_var(key) }
}

#[test]
#[serial]
fn daemon_syslog_emits_message() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sock");
    let server = UnixDatagram::bind(&path).unwrap();
    set_env_var("OC_RSYNC_SYSLOG_PATH", &path);
    init_logging(None, None, true, false, false);
    warn!(target: "test", "daemon syslog");
    let mut buf = [0u8; 256];
    let (n, _) = server.recv_from(&mut buf).unwrap();
    let msg = std::str::from_utf8(&buf[..n]).unwrap();
    let expected = format!("<12>rsync[{}]: daemon syslog", std::process::id());
    assert_eq!(msg, expected);
    remove_env_var("OC_RSYNC_SYSLOG_PATH");
}
