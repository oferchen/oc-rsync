use compress::{negotiate_codec, Codec, Compressor, Decompressor, Zlib, Zstd};

const DATA: &[u8] = b"The quick brown fox jumps over the lazy dog";

#[test]
fn zlib_roundtrip() {
    let codec = Zlib;
    let compressed = codec.compress(DATA).expect("compress");
    let decompressed = codec.decompress(&compressed).expect("decompress");
    assert_eq!(DATA, decompressed.as_slice());
}

#[test]
fn zstd_roundtrip() {
    let codec = Zstd;
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
