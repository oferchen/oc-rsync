// crates/cli/tests/block_size.rs

use assert_cmd::Command;
use tempfile::tempdir;
mod common;
use common::parse_literal;

#[test]
fn block_size_literal_data_matches() {
    let cases = [
        ("1k", 1 << 10),
        ("2K", 2 << 10),
        ("4M", 4 << 20),
        ("512", 512),
    ];

    for (arg, expected) in cases {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let src_file = src_dir.join("file.bin");
        let dst_file = dst_dir.join("file.bin");

        let size = 8 << 20;
        let mut basis = vec![0u8; size];
        for (i, b) in basis.iter_mut().enumerate() {
            *b = (i % 256) as u8;
        }
        let mut target = basis.clone();
        let off = size / 2;
        target[off..off + expected].fill(0xFF);
        std::fs::write(&src_file, &target).unwrap();
        std::fs::write(&dst_file, &basis).unwrap();

        let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
        cmd.args([
            "--stats",
            "--recursive",
            "--block-size",
            arg,
            "--no-whole-file",
            "--checksum",
        ]);
        cmd.arg(format!("{}/", src_dir.display()));
        cmd.arg("--");
        cmd.arg(dst_dir.to_str().unwrap());
        let output = cmd.output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let literal = parse_literal(&stdout);
        assert_eq!(literal, expected, "block-size {arg}");
    }
}
