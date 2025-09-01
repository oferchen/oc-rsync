// crates/engine/tests/filter.rs
use compress::available_codecs;
use engine::{sync, SyncOptions};
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn excluded_paths_are_skipped() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("include.txt"), b"include").unwrap();
    fs::write(src.join("skip.txt"), b"skip").unwrap();

    let mut visited = HashSet::new();
    let rules = parse("- skip.txt", &mut visited, 0).unwrap();
    let matcher = Matcher::new(rules);

    sync(
        &src,
        &dst,
        &matcher,
        &available_codecs(),
        &SyncOptions::default(),
    )
    .unwrap();

    assert!(dst.join("include.txt").exists());
    assert!(!dst.join("skip.txt").exists());
}
