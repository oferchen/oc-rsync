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

    let (stderr, truncated) = transport.stderr();
    assert!(truncated, "stderr not truncated");
    assert_eq!(stderr.len(), LIMIT);
}
