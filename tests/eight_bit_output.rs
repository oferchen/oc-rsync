// tests/eight_bit_output.rs
use assert_cmd::Command as TestCommand;
use std::fs;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[cfg(unix)]
fn invalid_name() -> std::ffi::OsString {
    std::ffi::OsString::from_vec(vec![b'f', 0xff, b'f'])
}

#[cfg(unix)]
#[test]
fn non_ascii_filename_output_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let fname = invalid_name();
    fs::write(src_dir.join(&fname), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    fs::create_dir_all(&dst_oc).unwrap();
    let output = TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--recursive")
        .arg("--out-format=%o%n")
        .arg(src_dir.as_os_str())
        .arg(dst_oc.as_os_str())
        .output()
        .unwrap();
    assert!(output.status.success());
    let ours_line = output
        .stdout
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send"))
        .unwrap()
        .to_vec();

    let expected = fs::read("tests/fixtures/rsync-send-nonascii-default.txt").unwrap();
    let expected_line = expected.split(|&b| b == b'\n').next().unwrap().to_vec();

    assert_eq!(ours_line, expected_line);
}

#[cfg(unix)]
#[test]
fn non_ascii_filename_eight_bit_output_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let fname = invalid_name();
    fs::write(src_dir.join(&fname), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    fs::create_dir_all(&dst_oc).unwrap();
    let output = TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--recursive")
        .arg("--8-bit-output")
        .arg("--out-format=%o%n")
        .arg(src_dir.as_os_str())
        .arg(dst_oc.as_os_str())
        .output()
        .unwrap();
    assert!(output.status.success());
    let ours_line = output
        .stdout
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send"))
        .unwrap()
        .to_vec();

    let expected = fs::read("tests/fixtures/rsync-send-nonascii-8bit.txt").unwrap();
    let expected_line = expected.split(|&b| b == b'\n').next().unwrap().to_vec();

    assert_eq!(ours_line, expected_line);
}

#[cfg(unix)]
#[test]
fn non_ascii_src_arg_eight_bit_output_matches_rsync() {
    let tmp = tempdir().unwrap();
    let fname = invalid_name();
    let src_file = tmp.path().join(&fname);
    fs::write(&src_file, b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    fs::create_dir_all(&dst_oc).unwrap();
    let output = TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--8-bit-output")
        .arg(src_file.as_os_str())
        .arg(dst_oc.as_os_str())
        .output()
        .unwrap();
    assert!(output.status.success());
    let ours_line = output
        .stdout
        .split(|&b| b == b'\n')
        .find(|l| l.starts_with(b"send"))
        .unwrap()
        .to_vec();

    let expected = fs::read("tests/fixtures/rsync-send-nonascii-8bit.txt").unwrap();
    let expected_line = expected.split(|&b| b == b'\n').next().unwrap().to_vec();

    assert_eq!(ours_line, expected_line);
    assert!(dst_oc.join(&fname).exists());
}
