// tests/rsync_zlib.rs
use compress::Codec;
use protocol::{negotiate_version, CAP_CODECS, LATEST_VERSION};
use transport::{ssh::SshStdioTransport, Transport};

#[test]
fn rsync_client_falls_back_to_zlib() {
    let mut t = SshStdioTransport::spawn("rsync", ["--server", "."]).unwrap();

    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    negotiate_version(u32::from_be_bytes(buf)).unwrap();

    t.send(&CAP_CODECS.to_be_bytes()).unwrap();
    t.receive(&mut buf).unwrap();
    let caps = u32::from_be_bytes(buf);
    assert_eq!(caps & CAP_CODECS, 0);

    let negotiated = vec![Codec::Zlib];
    assert_eq!(negotiated, vec![Codec::Zlib]);
}
