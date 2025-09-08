// tests/block_size.rs
#![allow(clippy::needless_range_loop)]

use assert_cmd::Command;
use checksums::ChecksumConfigBuilder;
use compress::available_codecs;
use engine::{Op, SyncOptions, block_size, compute_delta, sync};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;
mod common;
use common::parse_literal;

#[test]
fn block_size_matches_upstream() {
    let data = fs::read_to_string("tests/golden/block_size/upstream_block_sizes.txt").unwrap();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let len: u64 = parts.next().unwrap().parse().unwrap();
        let expected: usize = parts.next().unwrap().parse().unwrap();
        assert_eq!(block_size(len), expected, "len={len}");
    }
}

#[test]
fn delta_block_size_matches_rsync() {
    for &block_size in &[1024usize, 2048usize, 4096usize] {
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
        let ops: Vec<Op> = compute_delta(
            &cfg,
            &mut basis_f,
            &mut target_f,
            block_size,
            usize::MAX,
            &SyncOptions::default(),
        )
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

        let stats = fs::read_to_string(format!(
            "tests/golden/block_size/delta_block_size_matches_rsync_bs{block_size}.stdout"
        ))
        .unwrap();
        let rsync_literal = parse_literal(&stats);
        assert_eq!(literal, rsync_literal);
        assert_eq!(literal, block_size);
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
    let ops: Vec<Op> = compute_delta(
        &cfg,
        &mut basis_f,
        &mut target_f,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
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

    let stats =
        fs::read_to_string("tests/golden/block_size/delta_block_size_large_file.stdout").unwrap();
    let rsync_literal = parse_literal(&stats);
    assert_eq!(literal, rsync_literal);
    assert_eq!(literal, block_size);
}

#[test]
fn delta_block_size_unaligned_edit() {
    let block_size = 1024usize;
    let size = 1 << 20;
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
    let off = block_size / 2;
    target[off..off + block_size].fill(0xEE);
    fs::write(&src_file, &target).unwrap();
    fs::write(&dst_file, &basis).unwrap();

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis_f = fs::File::open(&dst_file).unwrap();
    let mut target_f = fs::File::open(&src_file).unwrap();
    let ops: Vec<Op> = compute_delta(
        &cfg,
        &mut basis_f,
        &mut target_f,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
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

    let stats =
        fs::read_to_string("tests/golden/block_size/delta_block_size_unaligned_edit.stdout")
            .unwrap();
    let rsync_literal = parse_literal(&stats);
    assert_eq!(literal, rsync_literal);
    assert_eq!(literal, block_size * 2);
}

#[test]
fn delta_block_size_non_power_two() {
    let block_size = 1000usize;
    let size = 1 << 20;
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
    target[off..off + block_size].fill(0xBB);
    fs::write(&src_file, &target).unwrap();
    fs::write(&dst_file, &basis).unwrap();

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis_f = fs::File::open(&dst_file).unwrap();
    let mut target_f = fs::File::open(&src_file).unwrap();
    let ops: Vec<Op> = compute_delta(
        &cfg,
        &mut basis_f,
        &mut target_f,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
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

    let stats = fs::read_to_string("tests/golden/block_size/delta_block_size_non_power_two.stdout")
        .unwrap();
    let rsync_literal = parse_literal(&stats);
    assert!(literal >= rsync_literal);
}

#[test]
fn delta_block_size_smaller_file() {
    let block_size = 4096usize;
    let size = 1024usize;
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
    target[100..150].fill(0xCC);
    fs::write(&src_file, &target).unwrap();
    fs::write(&dst_file, &basis).unwrap();

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis_f = fs::File::open(&dst_file).unwrap();
    let mut target_f = fs::File::open(&src_file).unwrap();
    let ops: Vec<Op> = compute_delta(
        &cfg,
        &mut basis_f,
        &mut target_f,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
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

    let stats =
        fs::read_to_string("tests/golden/block_size/delta_block_size_smaller_file.stdout").unwrap();
    let rsync_literal = parse_literal(&stats);
    assert_eq!(literal, rsync_literal);
    assert_eq!(literal, size);
}

#[test]
fn sync_block_size_literal_matches_rsync() {
    let block_size = 1024usize;
    let size = 1 << 20;
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
    target[off..off + block_size].fill(0xFF);
    fs::write(&src_file, &target).unwrap();
    fs::write(&dst_file, &basis).unwrap();

    let stats = sync(
        &src_dir,
        &dst_dir,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            block_size,
            checksum: true,
            ..Default::default()
        },
    )
    .unwrap();
    let rsync_stats =
        fs::read_to_string("tests/golden/block_size/cli_block_size_matches_rsync.stdout").unwrap();
    let rsync_literal = parse_literal(&rsync_stats) as u64;
    assert_eq!(stats.literal_data, rsync_literal);
    assert_eq!(stats.literal_data, block_size as u64);
}

#[test]
fn cli_block_size_matches_rsync() {
    let block_size_str = "1k";
    let block_size = 1024usize;
    let size = 1 << 20;
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
    target[off..off + block_size].fill(0xFF);
    fs::write(&src_file, &target).unwrap();
    fs::write(&dst_file, &basis).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let dst_arg = dst_dir.to_str().unwrap();
    let stats =
        fs::read_to_string("tests/golden/block_size/cli_block_size_matches_rsync.stdout").unwrap();
    let expected_status: i32 =
        fs::read_to_string("tests/golden/block_size/cli_block_size_matches_rsync.exit")
            .unwrap()
            .trim()
            .parse()
            .unwrap();
    let rsync_literal = parse_literal(&stats);

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--stats")
        .arg("--recursive")
        .arg("--block-size")
        .arg(block_size_str)
        .arg("--no-whole-file")
        .arg("--checksum")
        .arg(&src_arg)
        .arg("--")
        .arg(dst_arg)
        .output()
        .unwrap();
    let our_status = ours.status.code();
    let our_stdout = String::from_utf8(ours.stdout).unwrap();
    let our_stderr = String::from_utf8(ours.stderr).unwrap();
    assert_eq!(
        our_status,
        Some(expected_status),
        "stdout: {our_stdout}\nstderr: {our_stderr}"
    );
    let our_literal = parse_literal(&our_stdout);

    assert_eq!(
        our_literal, rsync_literal,
        "stdout: {our_stdout}\nstderr: {our_stderr}"
    );
    assert_eq!(
        our_literal, block_size,
        "stdout: {our_stdout}\nstderr: {our_stderr}"
    );
}

#[test]
fn cli_block_size_errors_match_rsync() {
    let tmp = tempdir().unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let expected_code: i32 =
        fs::read_to_string("tests/golden/block_size/cli_block_size_errors_match_rsync.exit")
            .unwrap()
            .trim()
            .parse()
            .unwrap();
    let rsync_err =
        fs::read_to_string("tests/golden/block_size/cli_block_size_errors_match_rsync.stderr")
            .unwrap();
    fn sanitize(line: &str) -> &str {
        line.split_once(' ').map(|(_, rhs)| rhs).unwrap_or(line)
    }
    let rsync_lines: Vec<_> = rsync_err.lines().collect();
    let rsync_first = sanitize(rsync_lines[0]);
    let rsync_second_lhs = rsync_lines[1]
        .split_once(" at ")
        .map(|(lhs, _)| lhs)
        .unwrap_or(rsync_lines[1]);
    let rsync_second_prefix = sanitize(rsync_second_lhs);

    for args in [&["--block-size=1x"][..], &["-B1x"][..], &["-B", "1x"][..]] {
        let ours = Command::cargo_bin("oc-rsync")
            .unwrap()
            .args(args)
            .arg("/dev/null")
            .arg("--")
            .arg(dst.to_str().unwrap())
            .output()
            .unwrap();
        assert_eq!(ours.status.code(), Some(expected_code));
        let ours_err = String::from_utf8(ours.stderr).unwrap();
        let ours_lines: Vec<_> = ours_err.lines().collect();
        assert_eq!(rsync_first, sanitize(ours_lines[0]));
        let ours_second_lhs = ours_lines[1]
            .split_once(" at ")
            .map(|(lhs, _)| lhs)
            .unwrap_or(ours_lines[1]);
        assert_eq!(rsync_second_prefix, sanitize(ours_second_lhs));
    }
}

#[test]
fn cli_block_size_zero_errors_match_rsync() {
    let tmp = tempdir().unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let expected_code: i32 =
        fs::read_to_string("tests/golden/block_size/cli_block_size_zero_errors_match_rsync.exit")
            .unwrap()
            .trim()
            .parse()
            .unwrap();
    let rsync_err =
        fs::read_to_string("tests/golden/block_size/cli_block_size_zero_errors_match_rsync.stderr")
            .unwrap();
    fn sanitize(line: &str) -> &str {
        line.split_once(' ').map(|(_, rhs)| rhs).unwrap_or(line)
    }
    let rsync_lines: Vec<_> = rsync_err.lines().collect();
    let rsync_first = sanitize(rsync_lines[0]);
    let rsync_second_lhs = rsync_lines[1]
        .split_once(" at ")
        .map(|(lhs, _)| lhs)
        .unwrap_or(rsync_lines[1]);
    let rsync_second_prefix = sanitize(rsync_second_lhs);

    for args in [&["--block-size=0"][..], &["-B0"][..], &["-B", "0"][..]] {
        let ours = Command::cargo_bin("oc-rsync")
            .unwrap()
            .args(args)
            .arg("/dev/null")
            .arg("--")
            .arg(dst.to_str().unwrap())
            .output()
            .unwrap();
        assert_eq!(ours.status.code(), Some(expected_code));
        let ours_err = String::from_utf8(ours.stderr).unwrap();
        let ours_lines: Vec<_> = ours_err.lines().collect();
        assert_eq!(rsync_first, sanitize(ours_lines[0]));
        let ours_second_lhs = ours_lines[1]
            .split_once(" at ")
            .map(|(lhs, _)| lhs)
            .unwrap_or(ours_lines[1]);
        assert_eq!(rsync_second_prefix, sanitize(ours_second_lhs));
    }
}
