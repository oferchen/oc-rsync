// tests/clap_error_codes.rs
use assert_cmd::Command;
use predicates::str::contains;
use protocol::ExitCode;
use tempfile::tempdir;

#[test]
fn unsupported_option_returns_exit_code_unsupported() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--bad-option",
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(u8::from(ExitCode::Unsupported) as i32)
        .stderr(contains("rsync: --bad-option: unknown option"))
        .stderr(contains(
            "rsync error: requested action not supported (code 4)",
        ));
}

#[test]
fn invalid_numeric_value_is_usage_error() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--timeout=abc",
            src.path().to_str().unwrap(),
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(u8::from(ExitCode::SyntaxOrUsage) as i32)
        .stderr(contains("rsync error: syntax or usage error (code 1)"));
}

#[test]
fn conflicting_flags_are_usage_error() {
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
        .code(u8::from(ExitCode::SyntaxOrUsage) as i32)
        .stderr(contains("rsync error: syntax or usage error (code 1)"));
}

#[test]
fn missing_destination_is_usage_error() {
    let src = tempdir().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([src.path().to_str().unwrap()])
        .assert()
        .failure()
        .code(u8::from(ExitCode::SyntaxOrUsage) as i32)
        .stderr(contains("rsync error: syntax or usage error (code 1)"));
}
