use assert_cmd::cargo::cargo_bin;
use compress::{available_codecs, decode_codecs, encode_codecs};
use protocol::{LATEST_VERSION, MIN_VERSION};
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
    let exe = cargo_bin("rsync-rs");
    let mut child = Command::new(exe)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    // Send our version.
    stdin.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();

    // Receive negotiated version.
    let mut ver_buf = [0u8; 4];
    stdout.read_exact(&mut ver_buf).unwrap();
    assert_eq!(u32::from_be_bytes(ver_buf), LATEST_VERSION);

    // Send codec list.
    let codecs = available_codecs();
    let payload = encode_codecs(codecs);
    stdin.write_all(&[payload.len() as u8]).unwrap();
    stdin.write_all(&payload).unwrap();

    // Receive server codec list.
    let mut len = [0u8; 1];
    stdout.read_exact(&mut len).unwrap();
    let mut buf = vec![0u8; len[0] as usize];
    stdout.read_exact(&mut buf).unwrap();
    let server_codecs = decode_codecs(&buf).unwrap();
    assert_eq!(server_codecs, codecs);

    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn server_rejects_unsupported_version() {
    let exe = cargo_bin("rsync-rs");
    let mut child = Command::new(exe)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();

    // Send an unsupported version.
    let bad = (MIN_VERSION - 1) as u32;
    stdin.write_all(&bad.to_be_bytes()).unwrap();
    drop(stdin);

    let status = child.wait().unwrap();
    assert!(!status.success());
}
