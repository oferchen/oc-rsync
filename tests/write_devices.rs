// tests/write_devices.rs
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn write_devices_flag_parses() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--write-devices", "--help"])
        .assert()
        .success();
}

#[test]
fn write_devices_requires_flag() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("file");
    fs::write(&file, b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", file.to_str().unwrap(), "/dev/null"])
        .assert()
        .failure();

    let meta = fs::metadata("/dev/null").unwrap();
    assert!(meta.file_type().is_char_device());
}

#[test]
fn write_devices_parity() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("file");
    fs::write(&file, b"hi").unwrap();

    let rsync_out = StdCommand::new("rsync")
        .args([
            "--quiet",
            "--write-devices",
            file.to_str().unwrap(),
            "/dev/null",
        ])
        .output()
        .unwrap();
    assert!(rsync_out.status.success());
    let rsync_output = String::from_utf8_lossy(&rsync_out.stdout).to_string()
        + &String::from_utf8_lossy(&rsync_out.stderr);

    let ours_out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--write-devices",
            file.to_str().unwrap(),
            "/dev/null",
        ])
        .output()
        .unwrap();
    assert!(ours_out.status.success());
    let ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);

    assert_eq!(rsync_output, ours_output);

    let meta = fs::metadata("/dev/null").unwrap();
    assert!(meta.file_type().is_char_device());
}
