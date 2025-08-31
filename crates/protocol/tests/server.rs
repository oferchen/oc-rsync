// crates/protocol/tests/server.rs
#[cfg(feature = "blake3")]
use checksums::StrongHash;
use compress::{available_codecs, encode_codecs, Codec};
#[cfg(feature = "blake3")]
use protocol::CAP_BLAKE3;
use protocol::{Server, CAP_CODECS, CAP_LZ4, CAP_ZSTD, LATEST_VERSION, SUPPORTED_CAPS};
use std::io::Cursor;
use std::time::Duration;

#[test]
fn server_negotiates_version() {
    let local = available_codecs(None);
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0];
        v.extend_from_slice(&LATEST_VERSION.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, peer_codecs) = srv
        .handshake(LATEST_VERSION, SUPPORTED_CAPS, &local)
        .unwrap();
    assert_eq!(srv.version, LATEST_VERSION);
    assert_eq!(caps, SUPPORTED_CAPS);
    assert_eq!(peer_codecs, local);
    let expected = {
        let mut v = LATEST_VERSION.to_be_bytes().to_vec();
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        let mut out_frame = Vec::new();
        codecs_frame.encode(&mut out_frame).unwrap();
        v.extend_from_slice(&out_frame);
        v
    };
    assert_eq!(output, expected);
}

#[test]
fn server_accepts_legacy_version() {
    let legacy = LATEST_VERSION - 1;
    let payload = encode_codecs(&available_codecs(None));
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0];
        v.extend_from_slice(&legacy.to_be_bytes());
        v.extend_from_slice(&CAP_CODECS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, peer_codecs) = srv
        .handshake(LATEST_VERSION, SUPPORTED_CAPS, &available_codecs(None))
        .unwrap();
    assert_eq!(srv.version, legacy);
    assert_eq!(caps & CAP_CODECS, CAP_CODECS);
    assert_eq!(peer_codecs, available_codecs(None));
    let expected = {
        let mut v = legacy.to_be_bytes().to_vec();
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        let mut out_frame = Vec::new();
        codecs_frame.encode(&mut out_frame).unwrap();
        v.extend_from_slice(&out_frame);
        v
    };
    assert_eq!(output, expected);
}

#[test]
fn server_classic_versions() {
    let local = available_codecs(None);
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();

    for ver in 27u32..=32 {
        let mut input = Cursor::new({
            let mut v = vec![0];
            v.extend_from_slice(&ver.to_be_bytes());
            v.extend_from_slice(&CAP_CODECS.to_be_bytes());
            v.extend_from_slice(&codecs_buf);
            v
        });
        let mut output = Vec::new();
        let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
        srv.handshake(LATEST_VERSION, SUPPORTED_CAPS, &local)
            .unwrap();
        assert_eq!(srv.version, ver);
        let expected = {
            let mut v = ver.to_be_bytes().to_vec();
            v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
            let mut out_frame = Vec::new();
            codecs_frame.encode(&mut out_frame).unwrap();
            v.extend_from_slice(&out_frame);
            v
        };
        assert_eq!(output, expected);
    }
}

#[cfg(feature = "blake3")]
#[test]
fn server_negotiates_blake3() {
    let codecs = vec![Codec::Zlib];
    let payload = encode_codecs(&codecs);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0];
        v.extend_from_slice(&LATEST_VERSION.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, _) = srv
        .handshake(LATEST_VERSION, SUPPORTED_CAPS, &codecs)
        .unwrap();
    assert_eq!(caps & CAP_BLAKE3, CAP_BLAKE3);
    assert!(matches!(srv.mux.strong_hash, StrongHash::Blake3));
    assert!(matches!(srv.demux.strong_hash, StrongHash::Blake3));
}

#[test]
fn server_negotiates_zstd() {
    let local = vec![Codec::Zstd, Codec::Zlib];
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0];
        v.extend_from_slice(&LATEST_VERSION.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, _) = srv
        .handshake(LATEST_VERSION, SUPPORTED_CAPS, &local)
        .unwrap();
    assert_eq!(caps & CAP_ZSTD, CAP_ZSTD);
    assert_eq!(srv.mux.compressor, Codec::Zstd);
    assert_eq!(srv.demux.compressor, Codec::Zstd);
}

#[test]
fn server_negotiates_lz4() {
    let local = vec![Codec::Lz4, Codec::Zlib];
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0];
        v.extend_from_slice(&LATEST_VERSION.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, _) = srv
        .handshake(LATEST_VERSION, SUPPORTED_CAPS, &local)
        .unwrap();
    assert_eq!(caps & CAP_LZ4, CAP_LZ4);
    assert_eq!(srv.mux.compressor, Codec::Lz4);
    assert_eq!(srv.demux.compressor, Codec::Lz4);
}
