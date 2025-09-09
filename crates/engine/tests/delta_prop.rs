// crates/engine/tests/delta_prop.rs
use checksums::ChecksumConfigBuilder;
use engine::{Op, Result as EngineResult, SyncOptions, compute_delta};
use proptest::prelude::*;
use std::io::Cursor;

const BLOCK_SIZE: usize = 64;

fn apply_ops(basis: &[u8], ops: &[Op]) -> Vec<u8> {
    let mut out = Vec::new();
    for op in ops {
        match op {
            Op::Data(d) => out.extend_from_slice(d),
            Op::Copy { offset, len } => out.extend_from_slice(&basis[*offset..*offset + *len]),
        }
    }
    out
}

fn source_target_strategy() -> impl Strategy<Value = (Vec<u8>, Vec<u8>)> {
    use proptest::collection::vec;

    let empty = Just((Vec::new(), Vec::new())).boxed();

    let single_byte = any::<u8>()
        .prop_map(|b| (vec![b], vec![b.wrapping_add(1)]))
        .boxed();

    let block_boundary = vec(any::<u8>(), BLOCK_SIZE * 2)
        .prop_map(|src| {
            let mut tgt = src.clone();
            tgt[BLOCK_SIZE] = tgt[BLOCK_SIZE].wrapping_add(1);
            (src, tgt)
        })
        .boxed();

    let large = (
        vec(any::<u8>(), BLOCK_SIZE * 100..BLOCK_SIZE * 200),
        vec(any::<u8>(), BLOCK_SIZE * 100..BLOCK_SIZE * 200),
    )
        .boxed();

    let random = (
        vec(any::<u8>(), 0..BLOCK_SIZE * 4),
        vec(any::<u8>(), 0..BLOCK_SIZE * 4),
    )
        .boxed();

    prop_oneof![empty, single_byte, block_boundary, large, random]
}

proptest! {
    #[test]
    fn delta_roundtrip((source, target) in source_target_strategy()) {
        let cfg = ChecksumConfigBuilder::new().build();
        let opts = SyncOptions::default();
        let mut src = Cursor::new(source.clone());
        let mut tgt = Cursor::new(target.clone());

        let delta = compute_delta(&cfg, &mut src, &mut tgt, BLOCK_SIZE, 8 * 1024, &opts).unwrap();
        let ops: Vec<Op> = delta.collect::<EngineResult<Vec<_>>>().unwrap();
        let reconstructed = apply_ops(&source, &ops);
        assert_eq!(reconstructed, target);
    }
}
