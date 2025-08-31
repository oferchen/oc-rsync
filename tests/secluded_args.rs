use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn accepts_secluded_args() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst = dir.path().join("dst");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--secluded-args",
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn accepts_s_alias() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst = dir.path().join("dst");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "-s",
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn rsync_protect_args_env_enables_secluded() {
    use assert_cmd::cargo::cargo_bin;
    use std::os::unix::fs::PermissionsExt;

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
        .env("RSYNC_PROTECT_ARGS", "1")
        .args([
            "-e",
            rsh.to_str().unwrap(),
            "--rsync-path",
            &rsync_path,
            "-r",
            &src_spec,
            &dst_spec,
        ])
        .assert()
        .success();

    let logged = fs::read_to_string(&log).unwrap();
    assert!(logged.contains("--secluded-args"));
}
