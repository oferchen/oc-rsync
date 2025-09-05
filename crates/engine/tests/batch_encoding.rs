// crates/engine/tests/batch_encoding.rs
use engine::{decode_batch, encode_batch, Batch};

#[test]
fn golden_batch_roundtrip() {
    let batch = Batch {
        flist: vec![b"file".to_vec()],
        checksums: vec![b"chk".to_vec()],
        data: vec![b"data".to_vec()],
    };
    let encoded = encode_batch(&batch);
    let golden = include_bytes!("../../../tests/golden/batch/simple.batch");
    assert_eq!(encoded, golden);
    let decoded = decode_batch(golden).unwrap();
    assert_eq!(decoded, batch);
}
