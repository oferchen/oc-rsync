// crates/compress/tests/codecs.rs
use compress::{decode_codecs, encode_codecs, negotiate_codec, should_compress, Codec};

#[cfg(any(feature = "zlib", feature = "zstd"))]
use compress::{Compressor, Decompressor};

#[cfg(feature = "zlib")]
use compress::Zlib;

#[cfg(feature = "zstd")]
use compress::Zstd;

use std::io;
use std::path::Path;

#[cfg(any(feature = "zlib", feature = "zstd"))]
const DATA: &[u8] = b"The quick brown fox jumps over the lazy dog";

#[cfg(feature = "zlib")]
#[test]
fn zlib_roundtrip() {
    let codec = Zlib::default();
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

#[test]
fn negotiation_helper_picks_common_codec() {
    let local = [Codec::Zstd, Codec::Zlib];
    let remote = [Codec::Zlib];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::Zlib));
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
    let codecs = vec![Codec::Zlib, Codec::Zstd];
    let encoded = encode_codecs(&codecs);
    assert_eq!(encoded, vec![1, 4]);
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
