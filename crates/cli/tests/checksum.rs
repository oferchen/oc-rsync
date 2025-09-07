// crates/cli/tests/checksum.rs

use assert_cmd::Command;
use filetime::{FileTime, set_file_mtime};
use tempfile::tempdir;
mod common;
use common::parse_literal;

#[test]
fn checksum_transfers_when_timestamps_match() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst).unwrap();

    let src_file = src_dir.join("file.bin");
    let dst_file = dst.join("file.bin");

    let a = vec![0xAAu8; 1024];
    let b = vec![0xBBu8; 1024];
    std::fs::write(&src_file, &a).unwrap();
    std::fs::write(&dst_file, &b).unwrap();

    let mtime = FileTime::from_unix_time(1_600_000_000, 0);
    set_file_mtime(&src_file, mtime).unwrap();
    set_file_mtime(&dst_file, mtime).unwrap();

    let out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum",
            "--block-size",
            "1k",
            "--no-whole-file",
            "--stats",
            src_file.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let literal = parse_literal(std::str::from_utf8(&out.stdout).unwrap());
    assert!(literal > 0);
}
