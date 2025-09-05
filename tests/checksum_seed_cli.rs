// tests/checksum_seed_cli.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn checksum_seed_flag_transfers_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let src_file = src.join("a.txt");
    fs::write(&src_file, vec![0u8; 2048]).unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum-seed=1",
            src_file.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = fs::read(dst.join("a.txt")).unwrap();
    assert_eq!(out, vec![0u8; 2048]);
}
