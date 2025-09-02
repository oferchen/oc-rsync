// crates/engine/tests/batch_replay.rs
use std::fs;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn replay_is_deterministic() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let record_dst = tmp.path().join("record");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&record_dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();

    let batch = tmp.path().join("batch.log");
    sync(
        &src,
        &record_dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            write_batch: Some(batch.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    fs::write(src.join("file2"), b"later").unwrap();
    fs::write(src.join("file"), b"hello").unwrap();

    let dst1 = tmp.path().join("dst1");
    let dst2 = tmp.path().join("dst2");
    fs::create_dir_all(&dst1).unwrap();
    fs::create_dir_all(&dst2).unwrap();

    let stats1 = sync(
        &src,
        &dst1,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            read_batch: Some(batch.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    let stats2 = sync(
        &src,
        &dst2,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            read_batch: Some(batch),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(stats1, stats2);
    assert_eq!(
        fs::read(dst1.join("file")).unwrap(),
        fs::read(dst2.join("file")).unwrap()
    );
    assert!(!dst1.join("file2").exists());
    assert!(!dst2.join("file2").exists());
}
