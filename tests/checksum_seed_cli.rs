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
    fs::write(src.join("a.txt"), b"seeded").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--checksum",
            "--checksum-seed=1",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out = fs::read(dst.join("a.txt")).unwrap();
    assert_eq!(out, b"seeded");
}
