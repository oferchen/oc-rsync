// tests/interop/codec_negotiation.rs
#![cfg(feature = "interop")]

use compress::{Codec, negotiate_codec};

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
