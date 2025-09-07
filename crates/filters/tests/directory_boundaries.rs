// crates/filters/tests/directory_boundaries.rs
use filters::{Matcher, parse};
use std::collections::HashSet;

fn p(input: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(input, &mut v, 0).unwrap()
}

#[test]
fn star_slash_file() {
    let rules = p("+ */file.txt\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("dir/file.txt").unwrap());
    assert!(!matcher.is_included("dir/sub/file.txt").unwrap());
}

#[test]
fn class_slash_star() {
    let rules = p("+ [0-9]/*\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("1/file.txt").unwrap());
    assert!(!matcher.is_included("1/dir/file.txt").unwrap());
}

#[test]
fn char_class_first_level_only() {
    let rules = p("+ [0-9]/*\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("1/file.txt").unwrap());
    assert!(!matcher.is_included("1/2/file.txt").unwrap());
    assert!(!matcher.is_included("dir/1/file.txt").unwrap());
}

#[test]
fn dir_prefix_double_star() {
    let rules = p("+ dir*/**/keep[0-9].txt\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("dir/keep1.txt").unwrap());
    assert!(matcher.is_included("dir/sub/keep2.txt").unwrap());
    assert!(!matcher.is_included("adir/keep3.txt").unwrap());
    assert!(!matcher.is_included("dir/keep10.txt").unwrap());
}

#[test]
fn leading_double_star() {
    let rules = p("+ **/keep?.txt\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("keep1.txt").unwrap());
    assert!(matcher.is_included("dir/keep2.txt").unwrap());
    assert!(!matcher.is_included("keep10.txt").unwrap());
    assert!(!matcher.is_included("dir/sub/keep12.txt").unwrap());
}
