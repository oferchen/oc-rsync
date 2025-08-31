#[cfg(unix)]
use assert_cmd::cargo::cargo_bin;
#[cfg(unix)]
use assert_cmd::prelude::*;
#[cfg(unix)]
use assert_cmd::Command;
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::process::Command as StdCommand;
#[cfg(unix)]
use std::process::Stdio;
#[cfg(unix)]
use tempfile::tempdir;

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn ssh_remote_option_forwarded() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    fs::write(src_dir.join("file.txt"), b"data").unwrap();
    let dst_dir = dir.path().join("dst");

    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("oc-rsync"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let log = dir.path().join("args.log");
    let wrapper = dir.path().join("wrapper.sh");
    fs::write(
        &wrapper,
        b"#!/bin/sh\nlog=\"$1\"; shift\nprintf '%s\n' \"$@\" > \"$log\"\nexec \"$@\"\n",
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let rsync_path = format!(
        "{} {} {}",
        wrapper.display(),
        log.display(),
        remote_bin.display()
    );

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
            &rsync_path,
            "--remote-option",
            &format!("--log-file={}", dir.path().join("remote.log").display()),
            "-r",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let logged = fs::read_to_string(&log).unwrap();
    assert!(logged.contains("--log-file"));
}

#[cfg(unix)]
#[test]
#[ignore]
fn daemon_remote_option_forwarded() {
    let dir = tempdir().unwrap();
    let module_dir = dir.path().join("module");
    fs::create_dir(&module_dir).unwrap();

    let mut child = StdCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--daemon",
            "--module",
            &format!("data={}", module_dir.display()),
            "--port",
            "0",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let port = read_port(&mut child);

    let log = dir.path().join("daemon.log");
    let _t = cli::spawn_daemon_session(
        "127.0.0.1",
        "data",
        Some(port),
        None,
        true,
        None,
        None,
        None,
        &[format!("--log-file={}", log.display())],
        protocol::LATEST_VERSION,
        None,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("127.0.0.1 data"));
    let _ = child.kill();
}
