// tests/server.rs

use assert_cmd::cargo::cargo_bin;
use compress::{available_codecs, decode_codecs, encode_codecs};
use protocol::{
    ExitCode, Frame, FrameHeader, Message, Msg, Tag, CAP_CODECS, LATEST_VERSION, MIN_VERSION,
};
use std::convert::TryFrom;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
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
    let (mut src_reader, _) = src_session.into_inner().expect("into_inner");
    let (_, mut dst_writer) = dst_session.into_inner().expect("into_inner");
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

    let mut transcript = Vec::new();

    stdin.write_all(&[0]).unwrap();
    stdin.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();

    let mut ver_buf = [0u8; 4];
    stdout.read_exact(&mut ver_buf).unwrap();
    transcript.extend_from_slice(&ver_buf);
    assert_eq!(u32::from_be_bytes(ver_buf), 31);

    stdin.write_all(&CAP_CODECS.to_be_bytes()).unwrap();
    let mut cap_buf = [0u8; 4];
    stdout.read_exact(&mut cap_buf).unwrap();
    transcript.extend_from_slice(&cap_buf);
    assert_eq!(u32::from_be_bytes(cap_buf) & CAP_CODECS, CAP_CODECS);

    let codecs = available_codecs();
    let payload = encode_codecs(&codecs);
    let frame = Message::Codecs(payload).to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    stdin.write_all(&buf).unwrap();

    let mut hdr = [0u8; 8];
    stdout.read_exact(&mut hdr).unwrap();
    transcript.extend_from_slice(&hdr);
    let channel = u16::from_be_bytes([hdr[0], hdr[1]]);
    let tag = Tag::try_from(hdr[2]).unwrap();
    let msg = Msg::try_from(hdr[3]).unwrap();
    let len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let mut payload = vec![0u8; len];
    stdout.read_exact(&mut payload).unwrap();
    transcript.extend_from_slice(&payload);
    let frame = Frame {
        header: FrameHeader {
            channel,
            tag,
            msg,
            len: len as u32,
        },
        payload: payload.clone(),
    };
    let msg = Message::from_frame(frame, None).unwrap();
    let server_codecs = match msg {
        Message::Codecs(data) => decode_codecs(&data).unwrap(),
        _ => panic!("expected codecs message"),
    };
    assert_eq!(server_codecs, codecs);

    let expected = fs::read("tests/fixtures/server_handshake_success.bin").unwrap();
    assert_eq!(transcript, expected);

    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn server_handshake_parses_args() {
    let exe = cargo_bin("oc-rsync");
    let mut child = Command::new(exe)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    let mut transcript = Vec::new();

    stdin.write_all(b"--foo\0bar\0\0").unwrap();
    stdin.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();

    let mut ver_buf = [0u8; 4];
    stdout.read_exact(&mut ver_buf).unwrap();
    transcript.extend_from_slice(&ver_buf);

    stdin.write_all(&CAP_CODECS.to_be_bytes()).unwrap();
    let mut cap_buf = [0u8; 4];
    stdout.read_exact(&mut cap_buf).unwrap();
    transcript.extend_from_slice(&cap_buf);

    let codecs = available_codecs();
    let payload = encode_codecs(&codecs);
    let frame = Message::Codecs(payload).to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    stdin.write_all(&buf).unwrap();

    let mut hdr = [0u8; 8];
    stdout.read_exact(&mut hdr).unwrap();
    transcript.extend_from_slice(&hdr);
    let len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let mut payload = vec![0u8; len];
    stdout.read_exact(&mut payload).unwrap();
    transcript.extend_from_slice(&payload);

    let expected = fs::read("tests/fixtures/server_handshake_success.bin").unwrap();
    assert_eq!(transcript, expected);

    let _ = child.kill();
    let _ = child.wait();
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
    let mut stdout = child.stdout.take().unwrap();

    let bad = MIN_VERSION - 1;
    stdin.write_all(&[0]).unwrap();
    stdin.write_all(&bad.to_be_bytes()).unwrap();
    drop(stdin);

    let mut ver_buf = [0u8; 4];
    stdout.read_exact(&mut ver_buf).unwrap();
    let expected = fs::read("tests/fixtures/server_handshake_unsupported_version.bin").unwrap();
    assert_eq!(ver_buf.to_vec(), expected);

    let status = child.wait().unwrap();
    assert!(!status.success());
}

#[test]
fn server_exit_code_roundtrip() {
    let exe = cargo_bin("oc-rsync");
    let mut child = Command::new(exe)
        .arg("--server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    stdin.write_all(&[0]).unwrap();
    stdin.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();

    let mut ver_buf = [0u8; 4];
    stdout.read_exact(&mut ver_buf).unwrap();

    stdin.write_all(&CAP_CODECS.to_be_bytes()).unwrap();
    let mut cap_buf = [0u8; 4];
    stdout.read_exact(&mut cap_buf).unwrap();

    let codecs = available_codecs();
    let payload = encode_codecs(&codecs);
    let frame = Message::Codecs(payload).to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    stdin.write_all(&buf).unwrap();

    let exit_frame = Message::Data(vec![ExitCode::Partial.into()]).to_frame(0, None);
    let mut exit_buf = Vec::new();
    exit_frame.encode(&mut exit_buf).unwrap();
    stdin.write_all(&exit_buf).unwrap();
    drop(stdin);

    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(ExitCode::Partial as i32));
}

#[cfg(unix)]
#[test]
fn stock_rsync_interop_over_ssh() {
    if Command::new("rsync")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("skipping stock rsync ssh interop test: rsync not found");
        return;
    }

    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"ssh_interop").unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let oc = cargo_bin("oc-rsync");
    let status = Command::new("rsync")
        .arg("-e")
        .arg(rsh.to_str().unwrap())
        .arg(&src)
        .arg(format!("fake:{}", dst.display()))
        .arg("--rsync-path")
        .arg(oc.to_str().unwrap())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(fs::read(&dst).unwrap(), b"ssh_interop");
}

#[cfg(unix)]
#[test]
fn stock_rsync_interop_daemon() {
    if Command::new("rsync")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("skipping stock rsync daemon interop test: rsync not found");
        return;
    }

    let src_dir = tempdir().unwrap();
    let file = src_dir.path().join("file.txt");
    fs::write(&file, b"daemon_interop").unwrap();

    let mut child = Command::new(cargo_bin("oc-rsync"))
        .args([
            "--daemon",
            "--module",
            &format!("data={}", src_dir.path().display()),
            "--port",
            "0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let port: u16 = {
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line.trim().parse().unwrap()
    };

    let dst_dir = tempdir().unwrap();
    let status = Command::new("rsync")
        .arg(format!("rsync://127.0.0.1:{port}/data/file.txt"))
        .arg(dst_dir.path().to_str().unwrap())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        fs::read(dst_dir.path().join("file.txt")).unwrap(),
        b"daemon_interop"
    );

    let _ = child.kill();
    let _ = child.wait();
}
