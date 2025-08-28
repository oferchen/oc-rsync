use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn daemon_accepts_modules() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["daemon", "--module", "data=/tmp", "--module", "home=/home"]);
    cmd.assert()
        .success()
        .stdout(contains("data => /tmp"))
        .stderr("");
}

#[test]
fn probe_reports_version() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["probe"]);
    cmd.assert()
        .success()
        .stdout(contains("negotiated version"))
        .stderr("");
}

#[test]
fn probe_rejects_old_version() {
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["probe", "--peer-version", "1"]);
    cmd.assert().failure();
}
