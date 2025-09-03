// crates/engine/tests/compress.rs
use std::fs;

use compress::Codec;
use engine::{select_codec, sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn zlib_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"hello world").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &[Codec::Zlib],
        &SyncOptions {
            compress: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn zstd_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"hello world").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &[Codec::Zstd],
        &SyncOptions {
            compress: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn codec_selection_respects_options() {
    let opts = SyncOptions {
        compress: true,
        ..Default::default()
    };
    assert_eq!(
        select_codec(&[Codec::Zlib, Codec::Zstd], &opts),
        Some(Codec::Zstd)
    );
    assert_eq!(select_codec(&[Codec::Zlib], &opts), Some(Codec::Zlib));

    let opts = SyncOptions {
        compress: true,
        compress_choice: Some(vec![Codec::Zlib]),
        ..Default::default()
    };
    assert_eq!(
        select_codec(&[Codec::Zlib, Codec::Zstd], &opts),
        Some(Codec::Zlib)
    );

    let opts = SyncOptions {
        compress: true,
        compress_level: Some(0),
        ..Default::default()
    };
    assert_eq!(select_codec(&[Codec::Zstd], &opts), None);
}
