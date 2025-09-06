// crates/engine/tests/delete.rs
use compress::available_codecs;
use engine::{DeleteMode, SyncOptions, sync};
use filters::{Matcher, parse};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fn run_delete_filter(mode: DeleteMode) {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(dst.join("keep.txt"), vec![0u8; 2048]).unwrap();
    fs::write(dst.join("remove.txt"), vec![0u8; 2048]).unwrap();

    let mut visited = HashSet::new();
    let rules = parse("- keep.txt", &mut visited, 0).unwrap();
    let matcher = Matcher::new(rules);

    sync(
        &src,
        &dst,
        &matcher,
        &available_codecs(),
        &SyncOptions {
            delete: Some(mode.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("remove.txt").exists());

    fs::write(dst.join("keep.txt"), vec![0u8; 2048]).unwrap();
    fs::write(dst.join("remove.txt"), vec![0u8; 2048]).unwrap();
    sync(
        &src,
        &dst,
        &matcher,
        &available_codecs(),
        &SyncOptions {
            delete: Some(mode),
            delete_excluded: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(!dst.join("keep.txt").exists());
    assert!(!dst.join("remove.txt").exists());
}

#[test]
fn delete_after_respects_filters() {
    run_delete_filter(DeleteMode::After);
}

#[test]
fn delete_during_respects_filters() {
    run_delete_filter(DeleteMode::During);
}
