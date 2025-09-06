// crates/engine/tests/fuzzy.rs
use std::fs;

use compress::available_codecs;
use engine::{SyncOptions, fuzzy_match, sync};
use filters::Matcher;
use tempfile::tempdir;

#[test]
fn fuzzy_match_exact_stem() {
    let tmp = tempdir().unwrap();
    let target = tmp.path().join("file.txt");
    let candidate = tmp.path().join("file.old");
    fs::write(&candidate, b"old").unwrap();
    assert_eq!(fuzzy_match(&target).unwrap(), candidate);
}

#[test]
fn fuzzy_match_prefers_closest_name() {
    let tmp = tempdir().unwrap();
    let target = tmp.path().join("file.txt");
    let close = tmp.path().join("fike");
    let far = tmp.path().join("bike");
    fs::write(&close, b"close").unwrap();
    fs::write(&far, b"far").unwrap();
    assert_eq!(fuzzy_match(&target).unwrap(), close);
}

#[test]
fn fuzzy_match_case_insensitive() {
    let tmp = tempdir().unwrap();
    let target = tmp.path().join("FILE.txt");
    let candidate = tmp.path().join("file");
    fs::write(&candidate, b"data").unwrap();
    assert_eq!(fuzzy_match(&target).unwrap(), candidate);
}

#[test]
fn engine_sync_uses_fuzzy_match() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    fs::write(src_dir.join("file"), b"hello").unwrap();
    fs::write(dst_dir.join("file.old"), b"world").unwrap();
    fs::write(dst_dir.join("fike"), b"???").unwrap();
    sync(
        &src_dir,
        &dst_dir,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            fuzzy: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(fs::read(dst_dir.join("file")).unwrap(), b"hello");
}
