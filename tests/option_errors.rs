// tests/option_errors.rs
use assert_cmd::Command;
use predicates::str::contains;
use protocol::ExitCode;
use tempfile::tempdir;

#[test]
fn invalid_checksum_choice_returns_protocol_error() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum-choice=bogus",
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(u8::from(ExitCode::Protocol) as i32)
        .stderr(contains("unknown checksum bogus"));
}

#[test]
fn invalid_compress_choice_returns_protocol_error() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--compress-choice=bogus",
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(u8::from(ExitCode::Protocol) as i32)
        .stderr(contains("unknown codec bogus"));
}

#[test]
fn ipv4_and_ipv6_flags_conflict() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--ipv4",
            "--ipv6",
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("cannot be used with"));
}
