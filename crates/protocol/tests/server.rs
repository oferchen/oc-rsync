// crates/protocol/tests/server.rs
use compress::{available_codecs, encode_codecs, Codec};
use protocol::{
    ExitCode, Server, CAP_ACLS, CAP_CODECS, CAP_XATTRS, CAP_ZSTD, SUPPORTED_CAPS,
    SUPPORTED_PROTOCOLS, V31, V32,
};
use std::io::Cursor;
use std::time::Duration;

#[test]
fn server_negotiates_version() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, peer_codecs) = srv.handshake(latest, SUPPORTED_CAPS, &local).unwrap();
    assert_eq!(srv.version, latest);
    assert_eq!(caps, SUPPORTED_CAPS);
    assert_eq!(peer_codecs, local);
    let expected = {
        let mut v = latest.to_be_bytes().to_vec();
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
    let legacy = V32;
    let payload = encode_codecs(&available_codecs());
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&legacy.to_be_bytes());
        v.extend_from_slice(&CAP_CODECS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let latest = SUPPORTED_PROTOCOLS[0];
    let (caps, peer_codecs) = srv
        .handshake(latest, SUPPORTED_CAPS, &available_codecs())
        .unwrap();
    assert_eq!(srv.version, legacy);
    assert_eq!(caps & CAP_CODECS, CAP_CODECS);
    assert_eq!(peer_codecs, available_codecs());
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
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();

    for ver in [V31, V32] {
        let mut input = Cursor::new({
            let mut v = vec![0, 0];
            v.extend_from_slice(&ver.to_be_bytes());
            v.extend_from_slice(&CAP_CODECS.to_be_bytes());
            v.extend_from_slice(&codecs_buf);
            v
        });
        let mut output = Vec::new();
        let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
        let latest = SUPPORTED_PROTOCOLS[0];
        srv.handshake(latest, SUPPORTED_CAPS, &local).unwrap();
        assert_eq!(srv.version, ver);
        let expected = {
            let mut v = latest.to_be_bytes().to_vec();
            v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
            let mut out_frame = Vec::new();
            codecs_frame.encode(&mut out_frame).unwrap();
            v.extend_from_slice(&out_frame);
            v
        };
        assert_eq!(output, expected);
    }
}

#[test]
fn server_negotiates_zstd() {
    let local = vec![Codec::Zstd, Codec::Zlib];
    let payload = encode_codecs(&local);
    let codecs_frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut codecs_buf = Vec::new();
    codecs_frame.encode(&mut codecs_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&codecs_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, _) = srv.handshake(latest, SUPPORTED_CAPS, &local).unwrap();
    assert_eq!(caps & CAP_ZSTD, CAP_ZSTD);
    assert_eq!(srv.mux.compressor, Codec::Zstd);
    assert_eq!(srv.demux.compressor, Codec::Zstd);
}

#[test]
fn server_propagates_handshake_error() {
    let mut buf = Vec::new();
    protocol::Message::Error("fail".into())
        .to_frame(0, None)
        .encode(&mut buf)
        .unwrap();
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
    let err = srv.handshake(latest, SUPPORTED_CAPS, &[]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(srv.demux.take_remote_error().as_deref(), Some("fail"));
}

#[test]
fn server_propagates_handshake_exit_code() {
    let mut buf = Vec::new();
    protocol::Message::Data(vec![1])
        .to_frame(0, None)
        .encode(&mut buf)
        .unwrap();
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
    let err = srv.handshake(latest, SUPPORTED_CAPS, &[]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Other);
    assert!(matches!(
        srv.demux.take_exit_code(),
        Some(Ok(ExitCode::SyntaxOrUsage))
    ));
}

#[test]
fn server_parses_args_and_env() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = Vec::new();
        v.extend_from_slice(b"--foo\0bar\0\0X=1\0\0");
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let _ = srv
        .handshake(latest, SUPPORTED_CAPS, &local)
        .expect("handshake");
    assert_eq!(srv.args, vec!["--foo", "bar"]);
    assert_eq!(srv.env, vec![("X".into(), "1".into())]);
}

#[test]
fn server_negotiates_optional_features() {
    let latest = SUPPORTED_PROTOCOLS[0];
    let peer_caps = (CAP_ACLS | CAP_XATTRS).to_be_bytes();
    let mut input = Cursor::new({
        let mut v = vec![0, 0];
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&peer_caps);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let (caps, peer_codecs) = srv.handshake(latest, SUPPORTED_CAPS, &[]).unwrap();
    assert_eq!(caps & (CAP_ACLS | CAP_XATTRS), CAP_ACLS | CAP_XATTRS);
    assert_eq!(peer_codecs, vec![Codec::Zlib]);
    let expected = {
        let mut v = latest.to_be_bytes().to_vec();
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v
    };
    assert_eq!(output, expected);
}

#[test]
fn server_accepts_equals_in_arg() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = Vec::new();
        v.extend_from_slice(b"--opt=foo=bar\0\0");
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    srv.handshake(latest, SUPPORTED_CAPS, &local)
        .expect("handshake");
    assert_eq!(srv.args, vec!["--opt=foo=bar"]);
    assert!(srv.env.is_empty());
}

#[test]
fn server_rejects_invalid_env() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = Vec::new();
        v.extend_from_slice(b"--foo\0\0BADENV\0\0");
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let err = srv
        .handshake(latest, SUPPORTED_CAPS, &local)
        .expect_err("handshake should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn server_rejects_option_after_arg() {
    let local = available_codecs();
    let payload = encode_codecs(&local);
    let frame = protocol::Message::Codecs(payload.clone()).to_frame(0, None);
    let mut frame_buf = Vec::new();
    frame.encode(&mut frame_buf).unwrap();
    let latest = SUPPORTED_PROTOCOLS[0];
    let mut input = Cursor::new({
        let mut v = Vec::new();
        v.extend_from_slice(b"foo\0--bar\0\0");
        v.extend_from_slice(&latest.to_be_bytes());
        v.extend_from_slice(&SUPPORTED_CAPS.to_be_bytes());
        v.extend_from_slice(&frame_buf);
        v
    });
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));
    let err = srv
        .handshake(latest, SUPPORTED_CAPS, &local)
        .expect_err("handshake should fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn server_surfaces_progress_and_stats() {
    let mut input = Cursor::new(Vec::new());
    let mut output = Vec::new();
    let mut srv = Server::new(&mut input, &mut output, Duration::from_secs(30));

    let _rx = srv.demux.register_channel(0);

    let prog = protocol::Message::Progress(42).to_frame(0, None);
    let stats = protocol::Message::Stats(vec![1, 2, 3]).to_frame(0, None);

    srv.demux.ingest(prog).unwrap();
    srv.demux.ingest(stats).unwrap();

    assert_eq!(srv.take_progress(), vec![42]);
    assert_eq!(srv.take_stats(), vec![vec![1, 2, 3]]);
}
