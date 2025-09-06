// crates/engine/tests/remove_source.rs
use std::fs;

use compress::available_codecs;
use engine::{SyncOptions, sync};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn removes_source_files_after_transfer() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"hi").unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            remove_source_files: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(dst.join("file.txt").exists());
    assert!(!src.join("file.txt").exists());

    assert!(src.exists());
}
