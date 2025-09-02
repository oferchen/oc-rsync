// crates/filters/tests/list_files.rs
use filters::{parse_with_options, Matcher};
use std::collections::HashSet;
use std::fs;

#[test]
fn include_from_newline_vs_null() {
    let tmp = tempfile::tempdir().unwrap();
    let nl = tmp.path().join("nl");
    fs::write(&nl, "foo\nbar\n").unwrap();
    let nul = tmp.path().join("nul");
    fs::write(&nul, b"foo\0bar\0").unwrap();

    let mut v1 = HashSet::new();
    let rules_nl = parse_with_options(
        &format!("include-from {}\n- *\n", nl.display()),
        false,
        &mut v1,
        0,
    )
    .unwrap();
    let m_nl = Matcher::new(rules_nl);
    assert!(m_nl.is_included("foo").unwrap());
    assert!(m_nl.is_included("bar").unwrap());
    assert!(!m_nl.is_included("baz").unwrap());

    let mut v2 = HashSet::new();
    let rules_nul = parse_with_options(
        &format!("include-from {}\n- *\n", nul.display()),
        true,
        &mut v2,
        0,
    )
    .unwrap();
    let m_nul = Matcher::new(rules_nul);
    assert!(m_nul.is_included("foo").unwrap());
    assert!(m_nul.is_included("bar").unwrap());
    assert!(!m_nul.is_included("baz").unwrap());
}

#[test]
fn include_exclude_precedence() {
    let tmp = tempfile::tempdir().unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "a\nb\n").unwrap();
    let filter = format!("+ c\nexclude-from {}\n+ a\n- *\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0).unwrap();
    let m = Matcher::new(rules);
    assert!(m.is_included("c").unwrap());
    assert!(m.is_included("a").unwrap());
    assert!(!m.is_included("b").unwrap());
    assert!(!m.is_included("d").unwrap());
}
