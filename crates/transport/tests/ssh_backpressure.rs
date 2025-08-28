use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use transport::ssh::SshStdioTransport;

#[test]
fn backpressure_blocks_until_read() {
    // Use a local echo server implemented via `cat`.
    let transport = SshStdioTransport::spawn("sh", ["-c", "cat"]).expect("spawn");
    let (mut reader, mut writer) = transport.into_inner();

    let data = vec![0x55u8; 200_000]; // larger than typical pipe capacity
    let handle = thread::spawn({
        let data = data.clone();
        move || {
            // This will block until the reader consumes data.
            writer.write_all(&data).expect("write_all");
        }
    });

    // Give the writer some time to fill the pipe.
    thread::sleep(Duration::from_millis(100));
    assert!(
        !handle.is_finished(),
        "write completed without backpressure"
    );

    // Drain the echoed data to release the backpressure.
    let mut buf = vec![0u8; data.len()];
    let mut read = 0;
    while read < buf.len() {
        read += reader.read(&mut buf[read..]).expect("read");
    }

    handle.join().expect("join");
    assert_eq!(buf, data);
}
