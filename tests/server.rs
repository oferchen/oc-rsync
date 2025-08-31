// tests/server.rs

use assert_cmd::cargo::cargo_bin;
use compress::{available_codecs, decode_codecs, encode_codecs};
use protocol::{Frame, FrameHeader, Message, Msg, Tag, CAP_CODECS, LATEST_VERSION, MIN_VERSION};
use std::convert::TryFrom;
use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[cfg(unix)]
mod remote_utils;
#[cfg(unix)]
use remote_utils::{spawn_reader, spawn_writer};

#[cfg(unix)]
#[test]
fn server_remote_pair_reports_error() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    fs::write(&src, b"data").unwrap();

    let src_session = spawn_reader(&format!("cat {}", src.display()));
    let dst_session = spawn_writer("exec 0<&-; sleep 1");
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    let res = std::io::copy(&mut src_reader, &mut dst_writer);
    assert!(res.is_err());
}

#[test]
fn server_handshake_succeeds() {
    let exe = cargo_bin("oc-rsync");
    let mut child = Command::new(exe)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    stdin.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();

    let mut ver_buf = [0u8; 4];
    stdout.read_exact(&mut ver_buf).unwrap();
    assert_eq!(u32::from_be_bytes(ver_buf), LATEST_VERSION);

    stdin.write_all(&CAP_CODECS.to_be_bytes()).unwrap();
    let mut cap_buf = [0u8; 4];
    stdout.read_exact(&mut cap_buf).unwrap();
    assert_eq!(u32::from_be_bytes(cap_buf) & CAP_CODECS, CAP_CODECS);

    let codecs = available_codecs(None);
    let payload = encode_codecs(&codecs);
    let frame = Message::Codecs(payload).to_frame(0);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    stdin.write_all(&buf).unwrap();

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
        payload: payload.clone(),
    };
    let msg = Message::from_frame(frame).unwrap();
    let server_codecs = match msg {
        Message::Codecs(data) => decode_codecs(&data).unwrap(),
        _ => panic!("expected codecs message"),
    };
    assert_eq!(server_codecs, codecs);

    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn server_rejects_unsupported_version() {
    let exe = cargo_bin("oc-rsync");
    let mut child = Command::new(exe)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();

    let bad = (MIN_VERSION - 1) as u32;
    stdin.write_all(&bad.to_be_bytes()).unwrap();
    drop(stdin);

    let status = child.wait().unwrap();
    assert!(!status.success());
}
