use assert_cmd::Command;
use std::fs;
use std::io::{Read, Write};
use tempfile::tempdir;
use transport::ssh::SshStdioTransport;
use serial_test::serial;

#[test]
fn remote_to_remote_pipes_data() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"hello remote\n").unwrap();

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(&dst_file).unwrap();
    assert_eq!(out, b"hello remote\n");
}

#[test]
fn remote_pair_missing_host_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    // Missing host in source spec should yield an error before attempting connections
    cmd.args([":/tmp/src", "sh:/tmp/dst"]);
    cmd.assert().failure();
}

#[test]
fn remote_pair_missing_path_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    // Missing path in source spec should also fail
    cmd.args(["sh:", "sh:/tmp/dst"]);
    cmd.assert().failure();
}

#[test]
fn remote_to_remote_large_transfer() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("large_src.bin");
    let dst_file = dir.path().join("large_dst.bin");
    let data = vec![0x5Au8; 5 * 1024 * 1024];
    fs::write(&src_file, &data).unwrap();

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(dst_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn remote_to_remote_reports_errors() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    fs::write(&src_file, b"data").unwrap();

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    // Destination process closes its stdin and signals when that has happened
    let dst_session = SshStdioTransport::spawn(
        "sh",
        ["-c", "exec 0<&-; echo ready; sleep 1"],
    )
    .unwrap();

    let (mut src_reader, _) = src_session.into_inner();
    let (mut dst_reader, mut dst_writer) = dst_session.into_inner();

    // Ensure the destination has closed stdin before attempting to copy
    let mut ready = [0u8; 6];
    dst_reader.read_exact(&mut ready).unwrap();
    assert_eq!(&ready, b"ready\n");

    let err = std::io::copy(&mut src_reader, &mut dst_writer).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
}

#[test]
#[serial]
fn remote_pair_unresolvable_host_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    // Using an unresolvable host should fail quickly
    cmd.args(["nosuchhost.invalid:/tmp/src", "nosuchhost.invalid:/tmp/dst"]);
    cmd.assert().failure();
}

#[test]
fn remote_to_remote_different_block_sizes() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.bin");
    let dst_file = dir.path().join("dst.bin");
    let data = vec![0xA5u8; 64 * 1024 + 123];
    fs::write(&src_file, &data).unwrap();

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();

    let mut read_buf = vec![0u8; 1024];
    let mut write_buf = Vec::with_capacity(4096);
    loop {
        let n = src_reader.read(&mut read_buf).unwrap();
        if n == 0 {
            if !write_buf.is_empty() {
                dst_writer.write_all(&write_buf).unwrap();
            }
            break;
        }
        write_buf.extend_from_slice(&read_buf[..n]);
        if write_buf.len() >= 4096 {
            dst_writer.write_all(&write_buf).unwrap();
            write_buf.clear();
        }
    }
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(dst_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn remote_to_remote_partial_and_resume() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"hello world").unwrap();

    // Initial partial transfer of first 5 bytes
    let src_session = SshStdioTransport::spawn(
        "sh",
        [
            "-c",
            &format!("head -c 5 {} 2>/dev/null", src_file.display()),
        ],
    )
    .unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let partial = fs::read(&dst_file).unwrap();
    assert_eq!(partial, b"hello");

    // Resume transfer with remaining bytes
    let src_session = SshStdioTransport::spawn(
        "sh",
        [
            "-c",
            &format!("tail -c +6 {} 2>/dev/null", src_file.display()),
        ],
    )
    .unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat >> {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(&dst_file).unwrap();
    assert_eq!(out, b"hello world");
}

#[test]
fn remote_to_remote_failure_and_reconnect() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"network glitch test").unwrap();

    // Initial transfer fails because destination immediately closes
    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", "exec 0<&-; sleep 1"]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    let result = std::io::copy(&mut src_reader, &mut dst_writer);
    assert!(result.is_err());
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(!dst_file.exists());

    // Reconnect and complete transfer
    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out_src = fs::read(&src_file).unwrap();
    let out_dst = fs::read(&dst_file).unwrap();
    assert_eq!(out_src, out_dst);
}
