// crates/engine/tests/streaming.rs
use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn sync_large_file_streaming() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();

    let mut data = Vec::new();
    for i in 0..(1024 * 65) {
        data.push((i % 256) as u8);
    }
    fs::write(src.join("file.bin"), &data).unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions::default(),
    )
    .unwrap();
    let out = fs::read(dst.join("file.bin")).unwrap();
    assert_eq!(out, data);
}

#[test]
fn sync_very_large_file_streaming() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();

    let mut data = vec![0u8; 1 * 1024 * 1024];
    for (i, b) in data.iter_mut().enumerate() {
        *b = (i % 256) as u8;
    }
    fs::write(src.join("big.bin"), &data).unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions::default(),
    )
    .unwrap();
    let out = fs::read(dst.join("big.bin")).unwrap();
    assert_eq!(out, data);
}
