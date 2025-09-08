// crates/engine/tests/block_size.rs

use checksums::ChecksumConfigBuilder;
use engine::{compute_delta, Op, Stats, SyncOptions};
use std::io::Cursor;

#[test]
fn block_size_literal_stats() {
    let block_size = 1024usize;
    let len = block_size * 2;
    let mut basis = vec![0u8; len];
    for i in 0..len {
        basis[i] = (i % 256) as u8;
    }
    let mut target = basis.clone();
    let off = len / 2;
    target[off..off + block_size].fill(0x55);

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis_f = Cursor::new(basis);
    let mut target_f = Cursor::new(target);
    let ops: Vec<Op> = compute_delta(
        &cfg,
        &mut basis_f,
        &mut target_f,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
    .unwrap()
    .collect::<Result<_>>()
    .unwrap();

    let mut stats = Stats::default();
    for op in ops {
        match op {
            Op::Data(d) => stats.literal_data += d.len() as u64,
            Op::Copy { len, .. } => stats.matched_data += len as u64,
        }
    }

    assert_eq!(stats.literal_data, block_size as u64);
}

#[test]
fn block_size_unaligned_literal_stats() {
    let block_size = 1024usize;
    let len = block_size * 2;
    let mut basis = vec![0u8; len];
    for i in 0..len {
        basis[i] = (i % 256) as u8;
    }
    let mut target = basis.clone();
    let off = block_size / 2;
    target[off..off + block_size].fill(0xAA);

    let cfg = ChecksumConfigBuilder::new().build();
    let mut basis_f = Cursor::new(basis);
    let mut target_f = Cursor::new(target);
    let ops: Vec<Op> = compute_delta(
        &cfg,
        &mut basis_f,
        &mut target_f,
        block_size,
        usize::MAX,
        &SyncOptions::default(),
    )
    .unwrap()
    .collect::<Result<_>>()
    .unwrap();

    let mut stats = Stats::default();
    for op in ops {
        match op {
            Op::Data(d) => stats.literal_data += d.len() as u64,
            Op::Copy { len, .. } => stats.matched_data += len as u64,
        }
    }

    assert_eq!(stats.literal_data, (block_size * 2) as u64);
}
