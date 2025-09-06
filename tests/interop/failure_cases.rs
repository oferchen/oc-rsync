#![cfg(unix)]

use assert_cmd::cargo::cargo_bin;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command as StdCommand, Stdio};
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
fn daemon_connection_refused_matches_rsync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = "rsync://127.0.0.1:1/module/dst/";

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_spec, dst_spec])
        .output()
        .unwrap();
    let upstream = StdCommand::new("rsync")
        .args([&src_spec, dst_spec])
        .output()
        .unwrap();
    assert_eq!(ours.status.code(), upstream.status.code());
    let our_err = String::from_utf8_lossy(&ours.stderr);
    let up_err = String::from_utf8_lossy(&upstream.stderr);
    assert!(our_err.contains("Connection refused"));
    assert!(up_err.contains("Connection refused"));
    assert_eq!(ours.status.code(), Some(10));
}

#[test]
fn daemon_auth_failure_matches_rsync() {
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
             [data]\n  path = {}\n  auth users = test\n  secrets file = {}/secrets\n  read only = false\n",
            module_dir.display(),
            dir.path().display()
        ),
    )
    .unwrap();
    fs::write(dir.path().join("secrets"), "test:correct").unwrap();
    fs::set_permissions(dir.path().join("secrets"), fs::Permissions::from_mode(0o600)).unwrap();

    let mut child = StdCommand::new(cargo_bin("oc-rsync"))
        .args(["--daemon", "--no-detach", "--port", "0", "--config", conf.to_str().unwrap()])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);

    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("rsync://test@127.0.0.1:{port}/data/dst/");

    let pw = dir.path().join("wrong.pw");
    fs::write(&pw, "wrong").unwrap();
    fs::set_permissions(&pw, fs::Permissions::from_mode(0o600)).unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_spec, &dst_spec, &format!("--password-file={}", pw.display())])
        .output()
        .unwrap();
    let upstream = StdCommand::new("rsync")
        .args([&src_spec, &dst_spec, &format!("--password-file={}", pw.display())])
        .output()
        .unwrap();
    assert_eq!(ours.status.code(), upstream.status.code());
    let our_err = String::from_utf8_lossy(&ours.stderr);
    let up_err = String::from_utf8_lossy(&upstream.stderr);
    assert!(our_err.contains("did not see server greeting"));
    assert!(up_err.contains("did not see server greeting"));
    assert_eq!(ours.status.code(), Some(5));

    let _ = child.kill();
}
