// crates/filters/tests/char_class_retains_dirs.rs
use filters::{Matcher, parse};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fn p(input: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(input, &mut v, 0).unwrap()
}

#[test]
fn include_char_class_retains_dirs() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("1/2")).unwrap();
    fs::write(tmp.path().join("1/keep.txt"), b"k").unwrap();
    fs::write(tmp.path().join("1/2/keep.txt"), b"x").unwrap();

    let rules = p("+ [0-9]/*\n- *\n");
    let matcher = Matcher::new(rules).with_root(tmp.path());
    assert!(matcher.is_included("1/keep.txt").unwrap());
    assert!(!matcher.is_included("1/2/keep.txt").unwrap());
    assert!(matcher.is_included_with_dir("1").unwrap().0);
    assert!(!matcher.is_included_with_dir("1/2").unwrap().0);
}
