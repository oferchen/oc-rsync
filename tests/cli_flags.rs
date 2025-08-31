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
