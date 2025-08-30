use assert_cmd::cargo::cargo_bin;
use compress::{available_codecs, decode_codecs, encode_codecs, Codec};
use protocol::{Frame, FrameHeader, Message, Msg, Tag, CAP_CODECS, LATEST_VERSION};
use checksums::{strong_digest, StrongHash};
use engine::{select_codec, SyncOptions};
use std::convert::TryFrom;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

#[test]
fn modern_negotiates_blake3_and_zstd() {
    let exe = cargo_bin("rsync-rs");
    let mut child = Command::new(exe)
        .args(["--server", "--modern"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    // Exchange protocol versions.
    stdin.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    stdout.read_exact(&mut buf).unwrap();
    assert_eq!(u32::from_be_bytes(buf), LATEST_VERSION);

    // Advertise capability bitmask and read server response.
    stdin.write_all(&CAP_CODECS.to_be_bytes()).unwrap();
    stdout.read_exact(&mut buf).unwrap();
    assert_ne!(u32::from_be_bytes(buf) & CAP_CODECS, 0);

    // Send codec list as a frame.
    let payload = encode_codecs(available_codecs());
    let frame = Message::Codecs(payload).to_frame(0);
    let mut send = Vec::new();
    frame.encode(&mut send).unwrap();
    stdin.write_all(&send).unwrap();

    // Receive server codec list as a frame.
    let mut hdr = [0u8; 8];
    stdout.read_exact(&mut hdr).unwrap();
    let channel = u16::from_be_bytes([hdr[0], hdr[1]]);
    let tag = Tag::try_from(hdr[2]).unwrap();
    let msg = Msg::try_from(hdr[3]).unwrap();
    let len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let mut payload = vec![0u8; len];
    stdout.read_exact(&mut payload).unwrap();
    let frame = Frame {
        header: FrameHeader {
            channel,
            tag,
            msg,
            len: len as u32,
        },
        payload,
    };
    let msg = Message::from_frame(frame).unwrap();
    let server_codecs = match msg {
        Message::Codecs(data) => decode_codecs(&data).unwrap(),
        _ => panic!("expected codecs message"),
    };

    // Negotiation should prefer zstd when both peers support it.
    let negotiated = select_codec(&server_codecs, &SyncOptions { compress: true, ..Default::default() }).unwrap();
    assert_eq!(negotiated, Codec::Zstd);

    // BLAKE3 strong digests are 32 bytes.
    let digest = strong_digest(b"hello world", StrongHash::Blake3);
    assert_eq!(digest.len(), 32);

    let status = child.wait().unwrap();
    assert!(status.success());
}

