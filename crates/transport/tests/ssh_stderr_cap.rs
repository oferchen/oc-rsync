use std::time::Duration;
use transport::ssh::SshStdioTransport;

use transport::Transport;

#[test]
fn caps_stderr_output() {
    const LIMIT: usize = 32 * 1024;
    let mut transport =
        SshStdioTransport::spawn("sh", ["-c", "head -c 100000 /dev/zero >&2"]).expect("spawn");

    // wait for process to exit by draining stdout
    let mut buf = [0u8; 1];
    let _ = transport.receive(&mut buf).unwrap();

    // stderr is captured asynchronously; wait until the reader thread finishes
    // writing to the buffer.
    let mut attempts = 0;
    loop {
        let (stderr, truncated) = transport.stderr();
        if truncated {
            assert_eq!(stderr.len(), LIMIT);
            break;
        }
        attempts += 1;
        assert!(attempts < 100, "stderr not truncated");
        std::thread::sleep(Duration::from_millis(10));
    }
}
