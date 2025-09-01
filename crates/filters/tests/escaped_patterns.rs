// crates/filters/tests/escaped_patterns.rs
use filters::{parse, Matcher};
use std::collections::HashSet;

fn p(s: &str) -> Matcher {
    let mut v = HashSet::new();
    Matcher::new(parse(s, &mut v, 0).unwrap())
}

#[test]
fn escaped_hash_and_space() {
    let m = p("+ foo\\ bar\\#baz\n- *\n");
    assert!(m.is_included("foo bar#baz").unwrap());
}

#[test]
fn escaped_trailing_space() {
    let m = p("+ file\\ \n- *\n");
    assert!(m.is_included("file ").unwrap());
}
