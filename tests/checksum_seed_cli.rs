// tests/checksum_seed_cli.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn checksum_seed_flag_transfers_files() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("a.txt");
    fs::write(&src_file, vec![0u8; 2048]).unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum-seed=1",
            src_file.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, vec![0u8; 2048]);

    let dst_file = dir.path().join("a.txt");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum-seed=1",
            src_file.to_str().unwrap(),
            dst_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = fs::read(&dst_file).unwrap();
    assert_eq!(out, vec![0u8; 2048]);
}
