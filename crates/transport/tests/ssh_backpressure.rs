// crates/transport/tests/ssh_backpressure.rs
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use transport::ssh::SshStdioTransport;

#[test]
fn backpressure_blocks_until_read() {
    let transport = SshStdioTransport::spawn("sh", ["-c", "cat"]).expect("spawn");
    let (mut reader, mut writer) = transport.into_inner().expect("into_inner");

    let data = vec![0x55u8; 200_000];
    let handle = thread::spawn({
        let data = data.clone();
        move || {
            writer.write_all(&data).expect("write_all");
        }
    });

    thread::sleep(Duration::from_millis(100));
    assert!(
        !handle.is_finished(),
        "write completed without backpressure"
    );

    let mut buf = vec![0u8; data.len()];
    let mut read = 0;
    while read < buf.len() {
        read += reader.read(&mut buf[read..]).expect("read");
    }

    handle.join().expect("join");
    assert_eq!(buf, data);
}
