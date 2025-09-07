// crates/filters/tests/advanced_globs.rs
use filters::{MAX_BRACE_EXPANSIONS, Matcher, ParseError, parse};
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

#[test]
fn brace_expansion_with_step() {
    let m = p("+ file{1..5..2}.txt\n- *\n");
    assert!(m.is_included("file1.txt").unwrap());
    assert!(m.is_included("file3.txt").unwrap());
    assert!(m.is_included("file5.txt").unwrap());
    assert!(!m.is_included("file2.txt").unwrap());
}

#[test]
fn character_class_matching() {
    let m = p("+ file[[:digit:]].txt\n- *\n");
    assert!(m.is_included("file0.txt").unwrap());
    assert!(m.is_included("file9.txt").unwrap());
    assert!(!m.is_included("filea.txt").unwrap());
}

#[test]
fn negated_character_class_matching() {
    let m = p("+ file[![:digit:]].txt\n- *\n");
    assert!(m.is_included("filea.txt").unwrap());
    assert!(!m.is_included("file1.txt").unwrap());
}

#[test]
fn escaped_wildcards() {
    let m = p("+ file\\*name\\?\n- *\n");
    assert!(m.is_included("file*name?").unwrap());
    assert!(!m.is_included("fileXnameQ").unwrap());
}

#[test]
fn escaped_brackets() {
    let m = p("+ file\\[data\\].txt\n- *\n");
    assert!(m.is_included("file[data].txt").unwrap());
    assert!(!m.is_included("filea.txt").unwrap());
}

#[test]
fn single_star_does_not_cross_directories() {
    let m = p("+ *.txt\n- *\n");
    assert!(m.is_included("file.txt").unwrap());
    assert!(!m.is_included("dir/file.txt").unwrap());
}

#[test]
fn double_star_matches_any_depth() {
    let m = p("+ **/keep.txt\n- *\n");
    assert!(m.is_included("keep.txt").unwrap());
    assert!(m.is_included("dir/sub/keep.txt").unwrap());
}

#[test]
fn character_class_confined_to_segment() {
    let m = p("+ [![:digit:]]/*.txt\n- *\n");
    assert!(m.is_included("a/file.txt").unwrap());
    assert!(!m.is_included("a/b/file.txt").unwrap());
}

#[test]
fn brace_expansion_limit_range() {
    let mut v = HashSet::new();
    let pattern = format!("+ file{{1..{}}}.txt\n- *\n", MAX_BRACE_EXPANSIONS + 1);
    let err = match parse(&pattern, &mut v, 0) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(matches!(err, ParseError::TooManyExpansions));
}

#[test]
fn brace_expansion_limit_set() {
    let mut body = String::new();
    for i in 0..=MAX_BRACE_EXPANSIONS {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&i.to_string());
    }
    let pattern = format!("+ file{{{}}}.txt\n- *\n", body);
    let err = match parse(&pattern, &mut HashSet::new(), 0) {
        Ok(_) => panic!("expected error"),
        Err(e) => e,
    };
    assert!(matches!(err, ParseError::TooManyExpansions));
}
