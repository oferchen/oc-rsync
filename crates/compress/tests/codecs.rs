// crates/compress/tests/codecs.rs
use compress::{
    available_codecs, decode_codecs, encode_codecs, negotiate_codec, should_compress, Codec,
};

#[cfg(any(feature = "zlib", feature = "zstd", feature = "lz4"))]
use compress::{Compressor, Decompressor};

#[cfg(feature = "zlib")]
use compress::{Zlib, ZlibX};

#[cfg(feature = "zstd")]
use compress::Zstd;

#[cfg(feature = "lz4")]
use compress::Lz4;

use std::io;
use std::path::Path;

#[cfg(any(feature = "zlib", feature = "zstd", feature = "lz4"))]
const DATA: &[u8] = b"The quick brown fox jumps over the lazy dog";

#[cfg(feature = "zlib")]
#[test]
fn zlib_roundtrip() {
    let codec = Zlib::default();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[cfg(feature = "zlib")]
#[test]
fn zlibx_roundtrip() {
    let codec = ZlibX::default();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[cfg(feature = "zstd")]
#[test]
fn zstd_roundtrip() {
    let codec = Zstd::default();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[cfg(feature = "lz4")]
#[test]
fn lz4_roundtrip() {
    let codec = Lz4::new();
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[test]
fn negotiation_helper_picks_common_codec() {
    let local = [Codec::Zstd, Codec::Lz4, Codec::Zlib];
    let remote = [Codec::Lz4, Codec::Zlib];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::Lz4));
    let remote2 = [Codec::Zstd];
    assert_eq!(negotiate_codec(&[Codec::Lz4], &remote2), None);
}

#[test]
fn codec_from_byte_rejects_unknown() {
    let err = Codec::from_byte(99).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn encode_decode_roundtrip_and_error() {
    let mut codecs = Vec::new();
    let mut bytes = Vec::new();
    #[cfg(feature = "zlib")]
    {
        codecs.push(Codec::Zlib);
        bytes.push(1);
        codecs.push(Codec::Zlibx);
        bytes.push(2);
    }
    #[cfg(feature = "lz4")]
    {
        codecs.push(Codec::Lz4);
        bytes.push(3);
    }
    #[cfg(feature = "zstd")]
    {
        codecs.push(Codec::Zstd);
        bytes.push(4);
    }
    let encoded = encode_codecs(&codecs);
    assert_eq!(encoded, bytes);
    let decoded = decode_codecs(&encoded).expect("decode");
    assert_eq!(decoded, codecs);
    let err = decode_codecs(&[42]).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn should_compress_respects_default_list() {
    assert!(should_compress(Path::new("file.txt"), &[]));
    assert!(!should_compress(Path::new("archive.gz"), &[]));
    assert!(!should_compress(Path::new("IMAGE.JpG"), &[]));
}

#[test]
fn should_compress_handles_mixed_case_patterns() {
    assert!(!should_compress(
        Path::new("file.TXT"),
        &["tXt".to_string()]
    ));
}

#[test]
fn available_codecs_matches_features() {
    let mut expected = Vec::new();
    #[cfg(feature = "zstd")]
    expected.push(Codec::Zstd);
    #[cfg(feature = "lz4")]
    expected.push(Codec::Lz4);
    #[cfg(feature = "zlib")]
    {
        expected.push(Codec::Zlibx);
        expected.push(Codec::Zlib);
    }
    assert_eq!(available_codecs(), expected);
}
