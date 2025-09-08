// crates/compress/tests/codecs.rs
use compress::{
    Codec, available_codecs, compressor, decode_codecs, decompressor, encode_codecs,
    negotiate_codec, should_compress,
};

use std::collections::HashSet;
use std::io;
use std::path::Path;

#[cfg(any(feature = "zlib", feature = "zstd"))]
const DATA: &[u8] = b"The quick brown fox jumps over the lazy dog";

#[cfg(feature = "zlib")]
#[test]
fn zlib_roundtrip() {
    let comp = compressor(Codec::Zlib).expect("compressor");
    let mut compressed = Vec::new();
    let mut src = DATA;
    comp.compress(&mut src, &mut compressed).expect("compress");
    let decomp = decompressor(Codec::Zlib).expect("decompressor");
    let mut decompressed = Vec::new();
    let mut c_slice = compressed.as_slice();
    decomp
        .decompress(&mut c_slice, &mut decompressed)
        .expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[cfg(feature = "zlib")]
#[test]
fn zlibx_roundtrip() {
    let comp = compressor(Codec::ZlibX).expect("compressor");
    let mut compressed = Vec::new();
    let mut src = DATA;
    comp.compress(&mut src, &mut compressed).expect("compress");
    let decomp = decompressor(Codec::ZlibX).expect("decompressor");
    let mut decompressed = Vec::new();
    let mut c_slice = compressed.as_slice();
    decomp
        .decompress(&mut c_slice, &mut decompressed)
        .expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[cfg(feature = "zstd")]
#[test]
fn zstd_roundtrip() {
    let comp = compressor(Codec::Zstd).expect("compressor");
    let mut compressed = Vec::new();
    let mut src = DATA;
    comp.compress(&mut src, &mut compressed).expect("compress");
    let decomp = decompressor(Codec::Zstd).expect("decompressor");
    let mut decompressed = Vec::new();
    let mut c_slice = compressed.as_slice();
    decomp
        .decompress(&mut c_slice, &mut decompressed)
        .expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[test]
fn negotiate_codec_returns_common_codec() {
    let local = [Codec::Zstd, Codec::Zlib];
    let remote = [Codec::Zlib];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::Zlib));
}

#[test]
fn negotiate_codec_returns_none_without_overlap() {
    let local = [Codec::Zlib];
    let remote = [Codec::Zstd];
    assert_eq!(negotiate_codec(&local, &remote), None);
}

#[test]
fn codec_from_byte_rejects_unknown() {
    let err = Codec::from_byte(99).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert_eq!(err.to_string(), "unknown codec 99");
}

#[test]
fn encode_decode_roundtrip_and_invalid_bytes() {
    let mut codecs = Vec::new();
    let mut bytes = Vec::new();
    #[cfg(feature = "zlib")]
    {
        codecs.push(Codec::Zlib);
        bytes.push(1);
        codecs.push(Codec::ZlibX);
        bytes.push(2);
    }
    #[cfg(feature = "zstd")]
    {
        codecs.push(Codec::Zstd);
        bytes.push(4);
    }
    let mut encoded = encode_codecs(&codecs);
    assert_eq!(encoded, bytes);
    let decoded = decode_codecs(&encoded).expect("decode");
    assert_eq!(decoded, codecs);
    encoded.push(42);
    let err = decode_codecs(&encoded).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn should_compress_respects_default_list() {
    let skip = HashSet::new();
    assert!(should_compress(Path::new("file.txt"), &skip));
    assert!(!should_compress(Path::new("archive.gz"), &skip));
    assert!(!should_compress(Path::new("IMAGE.JpG"), &skip));
    assert!(should_compress(Path::new("archivegz"), &skip));
}

#[test]
fn should_compress_handles_mixed_case_patterns() {
    let skip = ["tXt".to_ascii_lowercase()]
        .into_iter()
        .collect::<HashSet<_>>();
    assert!(!should_compress(Path::new("file.TXT"), &skip));
    assert!(should_compress(Path::new("archive.gz"), &skip));
}

#[test]
fn should_compress_requires_dot_with_custom_patterns() {
    let skip = ["gz".to_string()].into_iter().collect::<HashSet<_>>();
    assert!(!should_compress(Path::new("archive.gz"), &skip));
    assert!(should_compress(Path::new("archivegz"), &skip));
}

#[test]
fn should_compress_requires_lowercase_patterns() {
    let skip = ["GZ".to_string()].into_iter().collect::<HashSet<_>>();
    assert!(should_compress(Path::new("archive.gz"), &skip));
}

#[test]
fn available_codecs_matches_features() {
    let mut expected = Vec::new();
    #[cfg(feature = "zstd")]
    expected.push(Codec::Zstd);
    #[cfg(feature = "zlib")]
    {
        expected.push(Codec::ZlibX);
        expected.push(Codec::Zlib);
    }
    assert_eq!(available_codecs(), expected);
}
