// tests/interop/remote_option.rs
#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command as StdCommand;
use std::process::Stdio;
use tempfile::tempdir;

fn read_port(child: &mut std::process::Child) -> u16 {
    use std::io::Read;
    let stdout = child.stdout.as_mut().unwrap();
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while stdout.read(&mut byte).unwrap() == 1 {
        if byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
    }
    String::from_utf8(buf).unwrap().trim().parse().unwrap()
}

#[test]
fn ssh_remote_option_matches_rsync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    fs::write(src_dir.join("file.txt"), b"data").unwrap();
    let dst_dir = dir.path().join("dst");
    fs::create_dir(&dst_dir).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(
        &rsh,
        b"#!/bin/sh\nshift\nexec /bin/sh -c \"$*\"\n",
    )
    .unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());

    let rr_log = dir.path().join("rr.log");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--remote-option",
            &format!("--log-file={}", rr_log.display()),
            "-r",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    fs::remove_dir_all(&dst_dir).unwrap();
    fs::create_dir(&dst_dir).unwrap();

    let rs_log = dir.path().join("rs.log");
    let status = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--remote-option",
            &format!("--log-file={}", rs_log.display()),
            "-r",
            &src_spec,
            &dst_spec,
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let rr_contents = fs::read_to_string(&rr_log).unwrap();
    let rs_contents = fs::read_to_string(&rs_log).unwrap();
    assert_eq!(rr_contents, rs_contents);
}

#[test]
fn daemon_remote_option_matches_rsync() {
    let dir = tempdir().unwrap();
    let module_dir = dir.path().join("module");
    fs::create_dir(&module_dir).unwrap();

    let conf = dir.path().join("rsyncd.conf");
    fs::write(
        &conf,
        format!(
            "uid = 0\n\
             gid = 0\n\
             use chroot = false\n\
             [data]\n  path = {}\n  read only = false\n",
            module_dir.display()
        ),
    )
    .unwrap();

    let mut child = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "--daemon",
            "--no-detach",
            "--port",
            "0",
            "--config",
            conf.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);

    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    fs::write(src_dir.join("file.txt"), b"data").unwrap();
    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("rsync://127.0.0.1:{port}/data/dst/");

    let rr_log = dir.path().join("rr.log");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--remote-option",
            &format!("--log-file={}", rr_log.display()),
            "-r",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let dst_path = module_dir.join("dst");
    fs::remove_dir_all(&dst_path).unwrap();
    fs::create_dir(&dst_path).unwrap();

    let rs_log = dir.path().join("rs.log");
    let status = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "--remote-option",
            &format!("--log-file={}", rs_log.display()),
            "-r",
            &src_spec,
            &dst_spec,
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let rr_contents = fs::read_to_string(&rr_log).unwrap();
    let rs_contents = fs::read_to_string(&rs_log).unwrap();
    assert_eq!(rr_contents, rs_contents);

    let _ = child.kill();
}
