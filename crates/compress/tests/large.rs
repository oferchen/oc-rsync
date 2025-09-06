// crates/compress/tests/large.rs

#[cfg(any(feature = "zlib", feature = "zstd"))]
use compress::{Compressor, Decompressor};

#[cfg(feature = "zlib")]
use compress::Zlib;

#[cfg(feature = "zstd")]
use compress::Zstd;

const LARGE_SIZE: usize = 10 * 1024 * 1024;

#[cfg(feature = "zlib")]
#[test]
fn zlib_large_roundtrip() {
    let data = vec![0u8; LARGE_SIZE];
    let codec = Zlib::default();
    let mut compressed = Vec::new();
    let mut src = data.as_slice();
    codec.compress(&mut src, &mut compressed).expect("compress");
    let mut decompressed = Vec::new();
    let mut comp_slice = compressed.as_slice();
    codec
        .decompress(&mut comp_slice, &mut decompressed)
        .expect("decompress");
    assert_eq!(decompressed.len(), data.len());
    assert_eq!(data, decompressed);
}

#[cfg(feature = "zstd")]
#[test]
fn zstd_large_roundtrip() {
    let data = vec![0u8; LARGE_SIZE];
    let codec = Zstd::default();
    let mut compressed = Vec::new();
    let mut src = data.as_slice();
    codec.compress(&mut src, &mut compressed).expect("compress");
    let mut decompressed = Vec::new();
    let mut comp_slice = compressed.as_slice();
    codec
        .decompress(&mut comp_slice, &mut decompressed)
        .expect("decompress");
    assert_eq!(decompressed.len(), data.len());
    assert_eq!(data, decompressed);
}
