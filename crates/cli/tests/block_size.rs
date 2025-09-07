// crates/cli/tests/block_size.rs

use assert_cmd::Command;
use tempfile::tempdir;
mod common;
use common::parse_literal;

#[test]
fn block_size_literal_data_matches() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("file.bin");
    let dst_file = dst_dir.join("file.bin");

    let size = 1 << 20;
    let mut basis = vec![0u8; size];
    for i in 0..size {
        basis[i] = (i % 256) as u8;
    }
    let mut target = basis.clone();
    let off = size / 2;
    target[off..off + 1024].fill(0xFF);
    std::fs::write(&src_file, &target).unwrap();
    std::fs::write(&dst_file, &basis).unwrap();

    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--stats",
            "--recursive",
            "--block-size",
            "1k",
            "--no-whole-file",
            "--checksum",
            format!("{}/", src_dir.display()).as_str(),
            "--",
            dst_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let literal = parse_literal(&stdout);
    assert_eq!(literal, 1024);
}
