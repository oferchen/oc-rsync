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

#[test]
fn brace_expansion_comma() {
    let m = p("+ file{a,b}.txt\n- *\n");
    assert!(m.is_included("filea.txt").unwrap());
    assert!(m.is_included("fileb.txt").unwrap());
    assert!(!m.is_included("filec.txt").unwrap());
}

#[test]
fn brace_expansion_range() {
    let m = p("+ file{1..3}.txt\n- *\n");
    assert!(m.is_included("file1.txt").unwrap());
    assert!(m.is_included("file2.txt").unwrap());
    assert!(m.is_included("file3.txt").unwrap());
    assert!(!m.is_included("file4.txt").unwrap());
}
