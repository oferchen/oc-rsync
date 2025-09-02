// crates/engine/tests/upstream_batch.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn parses_rsync_style_batch_file() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("file name"), b"hi").unwrap();
    fs::write(src.join("other"), b"bye").unwrap();

    let batch = tmp.path().join("batch.log");
    fs::write(&batch, "./file\\040name\nfiles_transferred=1\n").unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            read_batch: Some(batch),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(dst.join("file name").exists());
    assert!(!dst.join("other").exists());
}
