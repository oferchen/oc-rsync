// crates/filters/tests/existing_prune.rs
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn existing_skips_missing_files() {
    let tmp = tempdir().unwrap();
    fs::write(tmp.path().join("present"), b"").unwrap();
    let mut v = HashSet::new();
    let rules = parse("existing\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(tmp.path());
    assert!(matcher.is_included("present").unwrap());
    assert!(!matcher.is_included("absent").unwrap());
}

#[test]
fn prune_empty_dirs_removes_empty_chains() {
    let tmp = tempdir().unwrap();
    fs::create_dir(tmp.path().join("empty")).unwrap();
    fs::create_dir(tmp.path().join("only_excluded")).unwrap();
    fs::write(tmp.path().join("only_excluded/secret"), b"").unwrap();
    fs::create_dir(tmp.path().join("with_files")).unwrap();
    fs::write(tmp.path().join("with_files/file"), b"").unwrap();
    let mut v = HashSet::new();
    let rules = parse("prune-empty-dirs\n- secret\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(tmp.path());
    assert!(!matcher.is_included("empty").unwrap());
    assert!(!matcher.is_included("only_excluded").unwrap());
    assert!(matcher.is_included("with_files").unwrap());
}
