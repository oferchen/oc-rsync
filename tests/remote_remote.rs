use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
use transport::ssh::SshStdioTransport;

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
    // Destination process exits immediately, causing a broken pipe
    // Close stdin immediately so writes fail with BrokenPipe
    let dst_session = SshStdioTransport::spawn("sh", ["-c", "exec 0<&-; sleep 1"]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    let result = std::io::copy(&mut src_reader, &mut dst_writer);
    assert!(result.is_err());
}

#[test]
fn remote_pair_unresolvable_host_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    // Using an unresolvable host should fail quickly
    cmd.args(["nosuchhost.invalid:/tmp/src", "nosuchhost.invalid:/tmp/dst"]);
    cmd.assert().failure();
}
