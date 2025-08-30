// crates/transport/tests/ssh_stdio.rs
use transport::{ssh::SshStdioTransport, Transport};

#[test]
fn send_receive_via_ssh_stdio() {
    let mut transport = SshStdioTransport::spawn("sh", ["-c", "cat"]).expect("spawn");

    transport.send(b"ping").expect("send");

    let mut buf = [0u8; 4];
    let n = transport.receive(&mut buf).expect("receive");
    assert_eq!(n, 4);
    assert_eq!(&buf, b"ping");
}
