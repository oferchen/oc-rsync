// tests/compression_negotiation.rs
use assert_cmd::Command;
use compress::{Codec, available_codecs, encode_codecs, negotiate_codec};
use engine::{SyncOptions, select_codec};
use protocol::{SUPPORTED_CAPS, SUPPORTED_PROTOCOLS, Server};
use std::io::Cursor;
use std::time::Duration;
mod common;
use common::temp_env;

#[test]
fn compression_negotiation() {
    assert_eq!(
        negotiate_codec(&[Codec::Zstd, Codec::Zlib], &[Codec::Zlib]),
        Some(Codec::Zlib)
    );

    let opts = SyncOptions {
        compress: true,
        ..Default::default()
    };
    assert_eq!(
        select_codec(&[Codec::Zstd, Codec::Zlib], &opts),
        Some(Codec::Zstd)
    );

    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (_caps, peer_codecs) = srv.handshake(latest, SUPPORTED_CAPS, &local, None).unwrap();
    assert_eq!(peer_codecs, local);

    let _env = temp_env("LC_ALL", "C");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}
