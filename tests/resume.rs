// tests/resume.rs

use assert_cmd::prelude::*;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[cfg(unix)]
use assert_cmd::cargo::cargo_bin;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::io::{self, Read};
#[cfg(unix)]
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::{Child, Command as StdCommand, Stdio};
#[cfg(unix)]
use std::sync::mpsc;

#[cfg(unix)]
fn read_port(child: &mut Child) -> io::Result<u16> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout"))?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        let res: io::Result<u16> = loop {
            match stdout.read(&mut byte) {
                Ok(0) => {
                    break Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "daemon closed",
                    ))
                }
                Ok(1) => {
                    if byte[0] == b'\n' {
                        break String::from_utf8(buf)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                            .and_then(|s| {
                                s.trim()
                                    .parse()
                                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
                            });
                    }
                    buf.push(byte[0]);
                }
                Ok(_) => unreachable!(),
                Err(e) => break Err(e),
            }
        };
        let _ = tx.send(res);
    });
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(res) => res,
        Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "timed out waiting for daemon port",
        )),
        Err(e) => Err(io::Error::other(e)),
    }
}

#[cfg(unix)]
fn wait_for_daemon(port: u16) {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("daemon did not start");
}

#[cfg(unix)]
fn spawn_daemon(module_path: &std::path::Path) -> (Child, u16) {
    let module_path = fs::canonicalize(module_path).unwrap();
    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--no-detach",
            "--module",
            &format!("data={}", module_path.display()),
            "--port",
            "0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let port = read_port(&mut child).unwrap();
    wait_for_daemon(port);
    (child, port)
}

#[test]
fn partial_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'a'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    assert!(dst_dir.join("a.partial").exists());

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[test]
fn partial_dir_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dst_dir.join("partial");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'a'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--partial-dir",
            partial_dir.to_str().unwrap(),
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    assert!(partial_dir.join("a.txt").exists());

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args([
        "--partial",
        "--partial-dir",
        partial_dir.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
    assert!(!partial_dir.exists());
}

#[cfg(unix)]
#[test]
fn remote_nested_partial_dir_transfer_resumes_after_interrupt_daemon() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let module_dir = dir.path().join("dst");
    let partial_dir = module_dir.join("partial");
    fs::create_dir_all(src_dir.join("sub")).unwrap();
    fs::create_dir_all(partial_dir.join("sub")).unwrap();
    let data = vec![b'i'; 2_000_000];
    fs::write(src_dir.join("sub/a.txt"), &data).unwrap();
    fs::write(partial_dir.join("sub/a.txt"), &data[..100_000]).unwrap();

    let (mut child, port) = spawn_daemon(&module_dir);

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("rsync://127.0.0.1:{port}/data/");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--partial-dir",
            "partial",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let out = fs::read(module_dir.join("sub/a.txt")).unwrap();
    assert_eq!(out, data);
    assert!(!partial_dir.exists());

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn append_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'b'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let dest_file = dst_dir.join("a.txt");
    std::fs::write(&dest_file, &data[..100_000]).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--append", "--inplace", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dest_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn append_verify_restarts_on_mismatch() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'c'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--bwlimit", "10240", &src_arg, dst_dir.to_str().unwrap()])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(500));
    let _ = child.kill();
    let _ = child.wait();

    let entries: Vec<_> = std::fs::read_dir(&dst_dir).unwrap().collect();
    assert_eq!(entries.len(), 1);
    let dest_file = entries[0].as_ref().unwrap().path();
    let mut partial = std::fs::read(&dest_file).unwrap();
    if !partial.is_empty() {
        partial[0] ^= 1;
        std::fs::write(&dest_file, &partial).unwrap();
    }

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args([
        "--append-verify",
        "--inplace",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[test]
fn partial_restarts_on_mismatch() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'd'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    let partial_file = dst_dir.join("a.partial");
    assert!(partial_file.exists());
    let mut partial = std::fs::read(&partial_file).unwrap();
    if !partial.is_empty() {
        partial[0] ^= 1;
        std::fs::write(&partial_file, &partial).unwrap();
    }

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--partial", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[test]
fn append_resumes_partial_file() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'j'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut child = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--partial",
            "--bwlimit",
            "10240",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let _ = child.kill();
    let _ = child.wait();

    assert!(dst_dir.join("a.partial").exists());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--append", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[test]
fn rsync_resumes_oc_partial_with_append() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'f'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();
    let dest_file = dst_dir.join("a.txt");
    std::fs::write(&dest_file, &data[..100_000]).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--append",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = std::fs::read(dest_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn rsync_append_verify_restarts_on_mismatch() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'g'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();
    let dest_file = dst_dir.join("a.txt");
    let mut partial = data[..100_000].to_vec();
    if !partial.is_empty() {
        partial[0] ^= 1;
    }
    std::fs::write(&dest_file, &partial).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--append-verify",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = std::fs::read(dest_file).unwrap();
    assert_eq!(out, data);
}

#[test]
fn oc_resumes_rsync_partial_with_append() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'h'; 200_000];
    std::fs::write(src_dir.join("a.txt"), &data).unwrap();

    let dest_file = dst_dir.join("a.txt");
    std::fs::write(&dest_file, &data[..100_000]).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--append", "--inplace", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(dest_file).unwrap();
    assert_eq!(out, data);
}

#[cfg(unix)]
#[test]
fn remote_partial_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let data = vec![b'e'; 2_000_000];
    fs::write(src_dir.join("a.txt"), &data).unwrap();
    fs::write(dst_dir.join("a.partial"), &data[..100_000]).unwrap();

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("oc-rsync"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--rsync-path",
            remote_bin.to_str().unwrap(),
            "--partial",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
}

#[cfg(unix)]
#[test]
fn remote_partial_dir_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dst_dir.join("partial");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&partial_dir).unwrap();
    let data = vec![b'e'; 2_000_000];
    fs::write(src_dir.join("a.txt"), &data).unwrap();
    fs::write(partial_dir.join("a.txt"), &data[..100_000]).unwrap();

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("oc-rsync"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--rsync-path",
            remote_bin.to_str().unwrap(),
            "--partial",
            "--partial-dir",
            "partial",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, data);
    assert!(!partial_dir.exists());
}

#[cfg(unix)]
#[test]
fn remote_nested_partial_dir_transfer_resumes_after_interrupt() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dst_dir.join("partial");
    fs::create_dir_all(src_dir.join("sub")).unwrap();
    fs::create_dir_all(partial_dir.join("sub")).unwrap();
    let data = vec![b'i'; 2_000_000];
    fs::write(src_dir.join("sub/a.txt"), &data).unwrap();
    fs::write(partial_dir.join("sub/a.txt"), &data[..100_000]).unwrap();

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("oc-rsync"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--rsync-path",
            remote_bin.to_str().unwrap(),
            "--partial",
            "--partial-dir",
            "partial",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let out = fs::read(dst_dir.join("sub/a.txt")).unwrap();
    assert_eq!(out, data);
    assert!(!partial_dir.exists());
}
