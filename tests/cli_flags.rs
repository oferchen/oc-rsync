// tests/cli_flags.rs
use assert_cmd::Command;
use tempfile::NamedTempFile;

#[test]
fn eight_bit_output_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--8-bit-output", "--version"])
        .assert()
        .success();
}

#[test]
fn blocking_io_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--blocking-io", "--version"])
        .assert()
        .success();
}

#[test]
fn early_input_flag_accepts_file() {
    let file = NamedTempFile::new().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--early-input", file.path().to_str().unwrap(), "--version"])
        .assert()
        .success();
}

#[test]
fn protocol_flag_accepts_version() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--protocol=31", "--version"])
        .assert()
        .success();
}

#[test]
fn log_file_flag_accepts_path() {
    let file = NamedTempFile::new().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--log-file", file.path().to_str().unwrap(), "--version"])
        .assert()
        .success();
}

#[test]
fn fsync_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--fsync", "--version"])
        .assert()
        .success();
}

#[test]
fn fuzzy_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--fuzzy", "--version"])
        .assert()
        .success();
}

#[test]
fn fake_super_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--fake-super", "--version"])
        .assert()
        .success();
}

#[test]
fn mkpath_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--mkpath", "--version"])
        .assert()
        .success();
}

#[test]
fn trust_sender_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--trust-sender", "--version"])
        .assert()
        .success();
}
