// crates/filters/tests/basic.rs
use filters::{parse, Matcher};
use std::collections::HashSet;

fn m(input: &str) -> Matcher {
    let mut v = HashSet::new();
    Matcher::new(parse(input, &mut v, 0).unwrap())
}

#[test]
fn character_class_matches_digits() {
    let matcher = m("+ file[0-9].txt\n- *\n");
    assert!(matcher.is_included("file1.txt").unwrap());
    assert!(!matcher.is_included("filea.txt").unwrap());
}

#[test]
fn negated_class_excludes_digits() {
    let matcher = m("+ file[!0-9].txt\n- *\n");
    assert!(matcher.is_included("filea.txt").unwrap());
    assert!(!matcher.is_included("file1.txt").unwrap());
}

#[test]
fn double_star_with_class_spans_dirs() {
    let matcher = m("+ dir/**/log[0-9].txt\n- *\n");
    assert!(matcher.is_included("dir/log1.txt").unwrap());
    assert!(matcher.is_included("dir/a/b/log2.txt").unwrap());
    assert!(!matcher.is_included("dir/a/b/logx.txt").unwrap());
}
