use checksums::ChecksumConfigBuilder;
use compress::available_codecs;
use engine::{Op, SyncOptions, compute_delta, sync};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;
use super::common::parse_literal;

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
fn sync_block_size_large_file_stats_match_rsync() {
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
        fs::read_to_string("tests/golden/block_size/delta_block_size_large_file.stdout").unwrap();
    let rsync_literal = parse_literal(&rsync_stats) as u64;
    assert_eq!(stats.literal_data, rsync_literal);
    assert_eq!(stats.literal_data, block_size as u64);
}
