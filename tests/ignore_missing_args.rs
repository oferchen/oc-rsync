use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn ignore_missing_args_allows_missing_sources() {
    let dst = tempdir().unwrap();
    // Without the flag, syncing a missing path should fail
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "missing-src", dst.path().to_str().unwrap()])
        .assert()
        .failure();

    // With --ignore-missing-args, the command should succeed
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--ignore-missing-args",
            "missing-src",
            dst.path().to_str().unwrap(),
        ])
        .assert()
        .success();
}
