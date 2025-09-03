// tests/cli_flags.rs
use assert_cmd::Command;
use oc_rsync_cli::cli_command;
use std::process::Command as StdCommand;
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
fn outbuf_flag_accepts_modes() {
    for mode in ["N", "L", "B"] {
        Command::cargo_bin("oc-rsync")
            .unwrap()
            .args([&format!("--outbuf={mode}"), "--version"])
            .assert()
            .success();
    }
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
fn mkpath_missing_args_matches_rsync() {
    let rsync = StdCommand::new("rsync").arg("--mkpath").output().unwrap();
    let oc = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--mkpath")
        .output()
        .unwrap();
    assert_eq!(rsync.status.success(), oc.status.success());
}

#[test]
fn trust_sender_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--trust-sender", "--version"])
        .assert()
        .success();
}

#[test]
fn short_attribute_flags_are_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-p", "-o", "-g", "-t", "-l", "-D", "--version"])
        .assert()
        .success();
}

#[test]
fn remove_sent_files_alias_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--remove-sent-files", "--version"])
        .assert()
        .success();
}

#[test]
fn delete_flags_last_one_wins() {
    let matches = cli_command()
        .try_get_matches_from(["prog", "--delete-after", "--delete-before", "src", "dst"])
        .unwrap();
    assert!(matches.get_flag("delete_before"));
    assert!(!matches.get_flag("delete_after"));
}

#[test]
fn old_args_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--old-args", "--version"])
        .assert()
        .success();
}

#[test]
fn old_dirs_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--old-dirs", "--version"])
        .assert()
        .success();
}

#[test]
fn old_d_alias_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--old-d", "--version"])
        .assert()
        .success();
}
