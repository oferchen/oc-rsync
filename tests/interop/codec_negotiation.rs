// tests/interop/codec_negotiation.rs

use compress::{negotiate_codec, Codec};

#[test]
fn negotiate_zlib_only() {
    let local = [Codec::Zlib, Codec::Zstd];
    let remote = [Codec::Zlib];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::Zlib));
}

#[test]
fn negotiate_zstd_only() {
    let local = [Codec::Zlib, Codec::Zstd];
    let remote = [Codec::Zstd];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::Zstd));
}

#[test]
fn negotiate_lz4_priority() {
    let local = [Codec::Zstd, Codec::Lz4, Codec::Zlib];
    let remote = [Codec::Lz4, Codec::Zlib];
    assert_eq!(negotiate_codec(&local, &remote), Some(Codec::Lz4));
}
