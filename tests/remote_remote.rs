// tests/remote_remote.rs
use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command as StdCommand;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use transport::ssh::SshStdioTransport;
use wait_timeout::ChildExt;

fn wait_for<F>(mut condition: F)
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    while !condition() {
        if start.elapsed() > Duration::from_secs(1) {
            panic!("timed out waiting for condition");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn assert_same_tree(a: &Path, b: &Path) {
    fn walk(a: &Path, b: &Path) {
        let mut ents_a: Vec<_> = fs::read_dir(a)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        ents_a.sort();
        let mut ents_b: Vec<_> = fs::read_dir(b)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        ents_b.sort();
        assert_eq!(ents_a, ents_b, "directory entries differ");
        for name in ents_a {
            let pa = a.join(&name);
            let pb = b.join(&name);
            let ma = fs::symlink_metadata(&pa).unwrap();
            let mb = fs::symlink_metadata(&pb).unwrap();
            assert_eq!(
                ma.file_type(),
                mb.file_type(),
                "file type differs for {:?}",
                name
            );
            assert_eq!(
                ma.permissions().mode(),
                mb.permissions().mode(),
                "permissions differ for {:?}",
                name
            );
            if ma.file_type().is_file() {
                assert_eq!(
                    fs::read(&pa).unwrap(),
                    fs::read(&pb).unwrap(),
                    "file contents differ for {:?}",
                    name
                );
            } else if ma.file_type().is_dir() {
                walk(&pa, &pb);
            } else if ma.file_type().is_symlink() {
                assert_eq!(
                    fs::read_link(&pa).unwrap(),
                    fs::read_link(&pb).unwrap(),
                    "symlink target differs for {:?}",
                    name
                );
            }
        }
    }
    walk(a, b);
}

#[test]
fn remote_remote_via_ssh_paths() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"via_ssh").unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("fake:{}", src.display());
    let dst_spec = format!("fake:{}", dst.display());

    let rr_bin = cargo_bin("rsync-rs");
    let rr_dir = rr_bin.parent().unwrap();
    let path_env = format!("{}:{}", rr_dir.display(), std::env::var("PATH").unwrap());
    let status = StdCommand::new(&rr_bin)
        .env("PATH", path_env)
        .args([
            "--archive",
            "--rsh",
            rsh.to_str().unwrap(),
            &src_spec,
            &dst_spec,
        ])
        .status()
        .unwrap();
    assert!(status.success());

    assert_same_tree(&src, &dst);
}

#[test]
#[ignore]
fn remote_remote_via_daemon_paths() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"via_daemon").unwrap();
    std::os::unix::fs::symlink("file.txt", src.join("link.txt")).unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut daemon = StdCommand::new(cargo_bin("rsync-rs"))
        .args([
            "--daemon",
            "--module",
            &format!("src={}", src.display()),
            "--module",
            &format!("dst={}", dst.display()),
            "--port",
            &port.to_string(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));

    let src_url = format!("rsync://127.0.0.1:{}/src/", port);
    let dst_url = format!("rsync://127.0.0.1:{}/dst/", port);

    let status = StdCommand::new(cargo_bin("rsync-rs"))
        .args(["--archive", &src_url, &dst_url])
        .status()
        .unwrap();
    assert!(status.success());

    std::thread::sleep(Duration::from_millis(200));
    assert_same_tree(&src, &dst);

    let _ = daemon.kill();
    let _ = daemon.wait();
}

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
    let expected = b"hello remote\n".len() as u64;
    wait_for(|| {
        fs::metadata(&dst_file)
            .map(|m| m.len() == expected)
            .unwrap_or(false)
    });

    let out = fs::read(&dst_file).unwrap();
    assert_eq!(out, b"hello remote\n");
}

#[test]
fn remote_pair_missing_host_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args([":/tmp/src", "sh:/tmp/dst"]);
    cmd.assert().failure();
}

#[test]
fn remote_pair_missing_path_fails() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
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
    let expected = data.len() as u64;
    wait_for(|| {
        fs::metadata(&dst_file)
            .map(|m| m.len() == expected)
            .unwrap_or(false)
    });

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
    let dst_session = SshStdioTransport::spawn("sh", ["-c", "exec 0<&-; echo ready"]).unwrap();

    let (mut src_reader, _) = src_session.into_inner();
    let (mut dst_reader, mut dst_writer) = dst_session.into_inner();

    let mut ready = [0u8; 6];
    dst_reader.read_exact(&mut ready).unwrap();
    assert_eq!(&ready, b"ready\n");

    let err = std::io::copy(&mut src_reader, &mut dst_writer).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
}

#[test]
fn remote_to_remote_empty_file() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("empty.txt");
    let dst_file = dir.path().join("copy.txt");
    fs::File::create(&src_file).unwrap();

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    wait_for(|| dst_file.exists());

    let out = fs::read(&dst_file).unwrap();
    assert!(out.is_empty());
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
    let expected = data.len() as u64;
    wait_for(|| {
        fs::metadata(&dst_file)
            .map(|m| m.len() == expected)
            .unwrap_or(false)
    });

    let out = fs::read(dst_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn remote_to_remote_partial_and_resume() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"hello world").unwrap();

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
    wait_for(|| {
        fs::metadata(&dst_file)
            .map(|m| m.len() == 5)
            .unwrap_or(false)
    });

    let partial = fs::read(&dst_file).unwrap();
    assert_eq!(partial, b"hello");

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
    wait_for(|| {
        fs::metadata(&dst_file)
            .map(|m| m.len() == 11)
            .unwrap_or(false)
    });

    let out = fs::read(&dst_file).unwrap();
    assert_eq!(out, b"hello world");
}

#[test]
fn remote_partial_transfer_resumed_by_cli() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"hello").unwrap();

    let partial = dst_dir.join("a.partial");
    let src_session = SshStdioTransport::spawn(
        "sh",
        [
            "-c",
            &format!("head -c 2 {} 2>/dev/null", src_dir.join("a.txt").display()),
        ],
    )
    .unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", partial.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", "--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!partial.exists());
}

#[test]
fn remote_to_remote_failure_and_reconnect() {
    let dir = tempdir().unwrap();
    let src_file = dir.path().join("src.txt");
    let dst_file = dir.path().join("dst.txt");
    fs::write(&src_file, b"network glitch test").unwrap();

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session = SshStdioTransport::spawn("sh", ["-c", "exec 0<&-; echo ready"]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (mut dst_reader, mut dst_writer) = dst_session.into_inner();

    let mut ready = [0u8; 6];
    dst_reader.read_exact(&mut ready).unwrap();
    assert_eq!(&ready, b"ready\n");

    let err = std::io::copy(&mut src_reader, &mut dst_writer).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    drop(dst_writer);
    drop(src_reader);
    assert!(!dst_file.exists());

    let src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src_file.display())]).unwrap();
    let dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst_file.display())]).unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    let expected = fs::metadata(&src_file).unwrap().len();
    wait_for(|| {
        fs::metadata(&dst_file)
            .map(|m| m.len() == expected)
            .unwrap_or(false)
    });

    let out_src = fs::read(&src_file).unwrap();
    let out_dst = fs::read(&dst_file).unwrap();
    assert_eq!(out_src, out_dst);
}

#[test]
#[ignore]
fn remote_remote_via_rsh_matches_rsync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_rr = dir.path().join("dst_rr");
    let dst_rsync = dir.path().join("dst_rsync");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst_rr).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let file = src.join("file.txt");
    fs::write(&file, b"via_rsh").unwrap();
    let mut perm = fs::metadata(&file).unwrap().permissions();
    perm.set_mode(0o600);
    fs::set_permissions(&file, perm).unwrap();
    std::os::unix::fs::symlink("file.txt", src.join("link.txt")).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("fake:{}", src.display());
    let dst_rr_spec = format!("fake:{}", dst_rr.display());
    let dst_rsync_spec = format!("fake:{}", dst_rsync.display());

    let rr_bin = cargo_bin("rsync-rs");
    let rr_dir = rr_bin.parent().unwrap();
    let path_env = format!("{}:{}", rr_dir.display(), std::env::var("PATH").unwrap());
    let mut child_rr = StdCommand::new(&rr_bin)
        .env("PATH", path_env)
        .args([
            "--archive",
            "--rsh",
            rsh.to_str().unwrap(),
            &src_spec,
            &dst_rr_spec,
        ])
        .spawn()
        .unwrap();
    let status_rr = child_rr
        .wait_timeout(Duration::from_secs(15))
        .unwrap()
        .expect("rsync-rs timed out");

    let mut child_rsync = StdCommand::new("rsync")
        .args([
            "--archive",
            "--rsh",
            rsh.to_str().unwrap(),
            &src_spec,
            &dst_rsync_spec,
        ])
        .spawn()
        .unwrap();
    let status_rsync = child_rsync
        .wait_timeout(Duration::from_secs(15))
        .unwrap()
        .expect("rsync timed out");

    assert_eq!(status_rr.code(), status_rsync.code());
    assert!(status_rr.success());

    assert_same_tree(&dst_rr, &dst_rsync);
}

#[test]
#[ignore]
fn remote_remote_via_rsync_urls_match_rsync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_rr = dir.path().join("dst_rr");
    let dst_rsync = dir.path().join("dst_rsync");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst_rr).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let file = src.join("file.txt");
    fs::write(&file, b"via_daemon").unwrap();
    let mut perm = fs::metadata(&file).unwrap().permissions();
    perm.set_mode(0o640);
    fs::set_permissions(&file, perm).unwrap();
    std::os::unix::fs::symlink("file.txt", src.join("link.txt")).unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let conf = dir.path().join("rsyncd.conf");
    fs::write(
        &conf,
        format!(
            "uid = root\n\
gid = root\n\
use chroot = false\n\
[src]\n  path = {}\n  read only = false\n\
[dst_rr]\n  path = {}\n  read only = false\n\
[dst_rsync]\n  path = {}\n  read only = false\n",
            src.display(),
            dst_rr.display(),
            dst_rsync.display()
        ),
    )
    .unwrap();

    let mut daemon = StdCommand::new("rsync")
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            &port.to_string(),
            "--config",
            conf.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(100));

    let src_url = format!("rsync://127.0.0.1:{}/src/", port);
    let dst_rr_url = format!("rsync://127.0.0.1:{}/dst_rr/", port);
    let dst_rsync_url = format!("rsync://127.0.0.1:{}/dst_rsync/", port);

    let rr_bin = cargo_bin("rsync-rs");
    let rr_dir = rr_bin.parent().unwrap();
    let path_env = format!("{}:{}", rr_dir.display(), std::env::var("PATH").unwrap());
    let mut child_rr = StdCommand::new(&rr_bin)
        .env("PATH", path_env)
        .args(["--archive", &src_url, &dst_rr_url])
        .spawn()
        .unwrap();
    let status_rr = child_rr
        .wait_timeout(Duration::from_secs(15))
        .unwrap()
        .expect("rsync-rs timed out");

    let mut child_rsync = StdCommand::new("rsync")
        .args(["--archive", &src_url, &dst_rsync_url])
        .spawn()
        .unwrap();
    let status_rsync = child_rsync
        .wait_timeout(Duration::from_secs(15))
        .unwrap()
        .expect("rsync timed out");

    assert_eq!(status_rr.code(), status_rsync.code());
    assert!(status_rr.success());

    std::thread::sleep(Duration::from_millis(50));
    assert_same_tree(&dst_rr, &dst_rsync);

    let _ = daemon.kill();
    let _ = daemon.wait();
}
