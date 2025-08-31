use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn accepts_secluded_args() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst = dir.path().join("dst");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--secluded-args",
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn accepts_s_alias() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst = dir.path().join("dst");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "-s",
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}
