// tests/modern.rs
#![cfg(feature = "blake3")]
use checksums::{strong_digest, StrongHash};
use compress::{available_codecs, Codec};
use engine::{select_codec, SyncOptions};

#[test]
fn modern_negotiates_blake3_and_zstd() {
    let codecs = available_codecs(true);
    let negotiated = select_codec(
        &codecs,
        &SyncOptions {
            compress: true,
            modern: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(negotiated, Codec::Zstd);
    let digest = strong_digest(b"hello world", StrongHash::Blake3, 0);
    assert_eq!(digest.len(), 32);
}

#[test]
fn modern_falls_back_without_compress() {
    let codecs = available_codecs(true);
    let negotiated = select_codec(
        &codecs,
        &SyncOptions {
            compress: false,
            modern: true,
            ..Default::default()
        },
    );
    assert!(negotiated.is_none());
}
