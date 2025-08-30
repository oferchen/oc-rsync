// tests/block_size.rs

use assert_cmd::Command;
use checksums::ChecksumConfigBuilder;
use engine::{compute_delta, Op};
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn parse_literal(stats: &str) -> usize {
    for line in stats.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("Literal data: ")
            .or_else(|| line.strip_prefix("Unmatched data: "))
        {
            let num_str = rest.split_whitespace().next().unwrap().replace(",", "");
            return num_str.parse().unwrap();
        }
    }
    panic!("no literal data in stats: {stats}");
}

#[test]
fn delta_block_size_matches_rsync() {
    for &block_size in &[1024usize, 2048usize] {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        let src_file = src_dir.join("file.bin");
        let dst_file = dst_dir.join("file.bin");

        let size = 1 << 20;
        let mut basis = vec![0u8; size];
        for i in 0..size {
            basis[i] = (i % 256) as u8;
        }
        let mut target = basis.clone();
        let off = size / 2;
        target[off..off + block_size].fill(0xFF);
        fs::write(&src_file, &target).unwrap();
        fs::write(&dst_file, &basis).unwrap();

        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis_f = fs::File::open(&dst_file).unwrap();
        let mut target_f = fs::File::open(&src_file).unwrap();
        let ops: Vec<Op> = compute_delta(&cfg, &mut basis_f, &mut target_f, block_size, usize::MAX)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let literal: usize = ops
            .iter()
            .map(|op| match op {
                Op::Data(d) => d.len(),
                _ => 0,
            })
            .sum();

        let output = StdCommand::new("rsync")
            .arg("--stats")
            .arg("--recursive")
            .arg("--block-size")
            .arg(block_size.to_string())
            .arg("--no-whole-file")
            .arg("--checksum")
            .arg(format!("{}/", src_dir.display()))
            .arg(dst_dir.to_str().unwrap())
            .output()
            .unwrap();
        assert!(output.status.success());
        let rsync_literal = parse_literal(std::str::from_utf8(&output.stdout).unwrap());
        assert_eq!(literal, rsync_literal);
        assert_eq!(literal, block_size);

        fs::write(&dst_file, &basis).unwrap();
        let src_arg = format!("{}/", src_dir.display());
        Command::cargo_bin("oc-rsync")
            .unwrap()
            .args([
                "--local",
                "--no-whole-file",
                "--checksum",
                "--block-size",
                &block_size.to_string(),
                &src_arg,
                dst_dir.to_str().unwrap(),
            ])
            .assert()
            .success();
    }
}

#[test]
fn delta_block_size_large_file() {
    let block_size = 8192usize;
    let size = 8 * 1024 * 1024;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let src_file = src_dir.join("file.bin");
    let dst_file = dst_dir.join("file.bin");

    let mut basis = vec![0u8; size];
    for i in 0..size {
        basis[i] = (i % 256) as u8;
    }
    let mut target = basis.clone();
    let off = size / 2;
    target[off..off + block_size].fill(0xAA);
    fs::write(&src_file, &target).unwrap();
    fs::write(&dst_file, &basis).unwrap();

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis_f = fs::File::open(&dst_file).unwrap();
    let mut target_f = fs::File::open(&src_file).unwrap();
    let ops: Vec<Op> = compute_delta(&cfg, &mut basis_f, &mut target_f, block_size, usize::MAX)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let literal: usize = ops
        .iter()
        .map(|op| match op {
            Op::Data(d) => d.len(),
            _ => 0,
        })
        .sum();

    let output = StdCommand::new("rsync")
        .arg("--stats")
        .arg("--recursive")
        .arg("--block-size")
        .arg(block_size.to_string())
        .arg("--no-whole-file")
        .arg("--checksum")
        .arg(format!("{}/", src_dir.display()))
        .arg(dst_dir.to_str().unwrap())
        .output()
        .unwrap();
    assert!(output.status.success());
    let rsync_literal = parse_literal(std::str::from_utf8(&output.stdout).unwrap());
    assert_eq!(literal, rsync_literal);
    assert_eq!(literal, block_size);
}
