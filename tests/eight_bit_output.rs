// tests/eight_bit_output.rs
use assert_cmd::Command as TestCommand;
use std::{fs, process::Command};
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[cfg(unix)]
fn invalid_name() -> std::ffi::OsString {
    std::ffi::OsString::from_vec(vec![b'f', 0xff, b'f'])
}

#[cfg(unix)]
#[test]
#[ignore]
fn non_ascii_filename_output_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let fname = invalid_name();
    fs::write(src_dir.join(&fname), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&dst_oc).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());

    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=%o %n",
            "--out-format=%o %n",
            &src_arg,
            dst_oc.to_str().unwrap(),
        ])
        .assert()
        .success();
    let ours = fs::read(&log).unwrap();
    let ours_line = ours
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send "))
        .unwrap()
        .to_vec();

    let output = Command::new("rsync")
        .args([
            "-r",
            "--out-format=%o %n",
            &src_arg,
            dst_rsync.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let theirs_line = output
        .stdout
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send "))
        .unwrap()
        .to_vec();

    assert_eq!(ours_line, theirs_line);
}

#[cfg(unix)]
#[test]
#[ignore]
fn non_ascii_filename_eight_bit_output_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let fname = invalid_name();
    fs::write(src_dir.join(&fname), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&dst_oc).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());

    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--8-bit-output",
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=%o %n",
            "--out-format=%o %n",
            &src_arg,
            dst_oc.to_str().unwrap(),
        ])
        .assert()
        .success();
    let ours = fs::read(&log).unwrap();
    let ours_line = ours
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send "))
        .unwrap()
        .to_vec();

    let output = Command::new("rsync")
        .args([
            "-r",
            "-8",
            "--out-format=%o %n",
            &src_arg,
            dst_rsync.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let theirs_line = output
        .stdout
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send "))
        .unwrap()
        .to_vec();

    assert_eq!(ours_line, theirs_line);
}
