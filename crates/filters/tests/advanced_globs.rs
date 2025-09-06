// crates/filters/tests/advanced_globs.rs
use filters::{Matcher, parse};
use std::collections::HashSet;

fn p(s: &str) -> Matcher {
    let mut v = HashSet::new();
    Matcher::new(parse(s, &mut v, 0).unwrap())
}

#[test]
fn bracket_set_matching() {
    let m = p("+ file[ab].txt\n- *\n");
    assert!(m.is_included("filea.txt").unwrap());
    assert!(m.is_included("fileb.txt").unwrap());
    assert!(!m.is_included("filec.txt").unwrap());
}

#[test]
fn bracket_negation_matching() {
    let m = p("+ file[!ab].txt\n- *\n");
    assert!(m.is_included("filec.txt").unwrap());
    assert!(!m.is_included("filea.txt").unwrap());
}

#[test]
fn double_star_precedence() {
    let m = p("- dir/**\n+ dir/**/keep.txt\n- *\n");
    assert!(!m.is_included("dir/sub/keep.txt").unwrap());
    let m2 = p("+ dir/**/keep.txt\n- dir/**\n- *\n");
    assert!(m2.is_included("dir/sub/keep.txt").unwrap());
}
