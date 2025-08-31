use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn ignore_missing_args_allows_missing_sources() {
    let dst = tempdir().unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "missing-src", dst.path().to_str().unwrap()])
        .assert()
        .failure();

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
