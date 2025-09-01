// crates/engine/tests/cdc.rs
#![cfg(feature = "blake3")]
use engine::cdc::chunk_bytes;

#[test]
#[ignore]
fn chunk_bytes_multi_gb() {
    let block = vec![0u8; 1024 * 1024];
    let iter = std::iter::repeat(block.as_slice()).take(2048);
    let chunks = chunk_bytes(iter, 64 * 1024, 128 * 1024, 256 * 1024);
    assert!(!chunks.is_empty());
}
