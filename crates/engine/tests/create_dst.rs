// crates/engine/tests/create_dst.rs
use std::fs;

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn creates_destination_when_missing() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), b"hi").unwrap();
    assert!(!dst.exists());
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions::default(),
    )
    .unwrap();
    assert!(dst.exists());
    assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hi");
}
