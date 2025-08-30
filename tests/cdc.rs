// tests/cdc.rs
use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn cdc_skips_renamed_file() {
    let home = tempdir().unwrap();
    std::env::set_var("HOME", home.path());

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let file_a = src.join("a.txt");
    fs::write(&file_a, b"hello world").unwrap();

    let opts = SyncOptions {
        cdc: true,
        modern: false,
        ..Default::default()
    };
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(false),
        &opts,
    )
    .unwrap();

    let file_b = src.join("b.txt");
    fs::rename(&file_a, &file_b).unwrap();

    let stats = sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(false),
        &opts,
    )
    .unwrap();
    assert_eq!(stats.bytes_transferred, 0);
    assert_eq!(fs::read(dst.join("b.txt")).unwrap(), b"hello world");
}
