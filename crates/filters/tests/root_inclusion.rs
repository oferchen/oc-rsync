// crates/filters/tests/root_inclusion.rs
use filters::{Matcher, parse};
use std::collections::HashSet;

fn m(input: &str) -> Matcher {
    let mut v = HashSet::new();
    Matcher::new(parse(input, &mut v, 0).unwrap())
}

#[test]
fn source_root_is_included_despite_global_exclude() {
    let matcher = m("- *\n");
    assert!(matcher.is_included("").unwrap());
    assert!(!matcher.is_included("something").unwrap());
}
