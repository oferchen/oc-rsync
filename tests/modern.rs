// tests/modern.rs

#![cfg(feature = "blake3")]
use checksums::{strong_digest, StrongHash};
use compress::{available_codecs, Codec, ModernCompress};
use engine::{select_codec, ModernHash, SyncOptions};

#[test]
fn modern_negotiates_blake3_and_zstd() {
    let codecs = available_codecs(Some(ModernCompress::Auto));
    let negotiated = select_codec(
        &codecs,
        &SyncOptions {
            compress: true,
            modern_compress: Some(ModernCompress::Auto),
            modern_hash: Some(ModernHash::Blake3),
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
    let codecs = available_codecs(Some(ModernCompress::Auto));
    let negotiated = select_codec(
        &codecs,
        &SyncOptions {
            compress: false,
            modern_compress: Some(ModernCompress::Auto),
            ..Default::default()
        },
    );
    assert!(negotiated.is_none());
}
