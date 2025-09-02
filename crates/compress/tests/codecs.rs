// crates/compress/tests/codecs.rs
use compress::{
    decode_codecs, encode_codecs, negotiate_codec, should_compress, Codec, Compressor,
    Decompressor, Zlib, ZlibX, Zstd,
};
use std::io;
use std::path::Path;

const DATA: &[u8] = b"The quick brown fox jumps over the lazy dog";

#[test]
fn zlib_roundtrip() {
    let codec = Zlib::default();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[test]
fn zlibx_roundtrip() {
    let codec = ZlibX::default();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[test]
fn zstd_roundtrip() {
    let codec = Zstd::default();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[test]
fn negotiation_helper_picks_common_codec() {
    let local = [Codec::Zstd, Codec::ZlibX, Codec::Zlib];
    let remote = [Codec::ZlibX, Codec::Zlib];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::ZlibX));
    let remote2 = [Codec::Zstd];
    assert_eq!(negotiate_codec(&[Codec::Zlib], &remote2), None);
}

#[test]
fn codec_from_byte_rejects_unknown() {
    let err = Codec::from_byte(99).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn encode_decode_roundtrip_and_error() {
    let codecs = vec![Codec::Zlib, Codec::ZlibX, Codec::Zstd];
    let encoded = encode_codecs(&codecs);
    assert_eq!(encoded, vec![1, 2, 4]);
    let decoded = decode_codecs(&encoded).expect("decode");
    assert_eq!(decoded, codecs);
    let err = decode_codecs(&[42]).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn should_compress_skips_matching_extensions() {
    assert!(should_compress(Path::new("file.txt"), &[]));
    assert!(!should_compress(
        Path::new("archive.gz"),
        &["gz".to_string()]
    ));
}
