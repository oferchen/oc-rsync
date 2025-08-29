use std::fs;
use std::io;
use std::thread::sleep;
use std::time::Duration;

use transport::{ssh::SshStdioTransport, Transport};

#[test]
fn no_zombie_after_drop() {
    let mut t = SshStdioTransport::spawn("sh", ["-c", "echo $$; read line"]).expect("spawn");

    // Read the PID line from the child process.
    let mut pid_bytes = Vec::new();
    loop {
        let mut buf = [0u8; 1];
        if t.receive(&mut buf).expect("receive") == 0 {
            panic!("EOF before pid");
        }
        if buf[0] == b'\n' {
            break;
        }
        pid_bytes.push(buf[0]);
    }
    let pid: u32 = std::str::from_utf8(&pid_bytes)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    // Drop the transport which should wait on the child process.
    drop(t);
    // Allow some time for the OS to reap the process.
    sleep(Duration::from_millis(100));

    // The process should no longer exist.
    let status_path = format!("/proc/{pid}/status");
    let err = fs::read_to_string(status_path).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}
