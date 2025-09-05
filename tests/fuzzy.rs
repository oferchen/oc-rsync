// tests/fuzzy.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn fuzzy_reports_error() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("file");
    fs::write(&src_file, b"hello").unwrap();
    fs::write(dst_dir.join("file.old"), b"world").unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--fuzzy",
            src_file.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not a directory"));
}
