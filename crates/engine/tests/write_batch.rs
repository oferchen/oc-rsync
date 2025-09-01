// crates/engine/tests/write_batch.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn writes_batch_file() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    let batch = tmp.path().join("batch.log");
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            write_batch: Some(batch.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    let log = fs::read_to_string(batch).unwrap();
    assert!(log.contains("files_transferred=1"));
    assert!(log.contains("bytes_transferred=2"));
}
