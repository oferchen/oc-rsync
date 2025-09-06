// crates/filters/tests/files_from.rs
use filters::{Matcher, parse, parse_with_options};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fn p(s: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0).unwrap()
}

#[test]
fn files_from_emulation() {
    let rules = p("+ foo\n+ bar\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}

#[test]
fn files_from_null_separated() {
    let input = b"foo\0bar\0";
    let mut rules = Vec::new();
    for part in input.split(|b| *b == 0) {
        if part.is_empty() {
            continue;
        }
        let pat = String::from_utf8_lossy(part);
        rules.extend(p(&format!("+ {}\n", pat)));
    }
    rules.extend(p("- *\n"));
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}

#[test]
fn files_from_vs_exclude_ordering() {
    let tmp = tempdir().unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "a\nb\n").unwrap();
    let filter = format!("files-from {}\n- a\n- *\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let m = Matcher::new(rules);
    assert!(m.is_included("b").unwrap());
    assert!(!m.is_included("a").unwrap());

    let filter_rev = format!("- a\nfiles-from {}\n- *\n", list.display());
    let mut v2 = HashSet::new();
    let rules_rev = parse_with_options(&filter_rev, false, &mut v2, 0, None).unwrap();
    let m_rev = Matcher::new(rules_rev);
    assert!(!m_rev.is_included("a").unwrap());
    assert!(m_rev.is_included("b").unwrap());
}

#[test]
fn files_from_directory_entries_imply_parents() {
    let tmp = tempdir().unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "a/b/\n").unwrap();
    let filter = format!("files-from {}\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let m = Matcher::new(rules);
    assert!(m.is_included("a").unwrap());
    assert!(m.is_included("a/b").unwrap());
    assert!(m.is_included("a/b/c").unwrap());
    assert!(!m.is_included("c").unwrap());
}

#[test]
fn files_from_mixed_file_dir_entries() {
    let tmp = tempdir().unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz\nqux/\n").unwrap();
    let filter = format!("files-from {}\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let m = Matcher::new(rules);
    assert!(m.is_included("foo").unwrap());
    assert!(m.is_included("foo/bar").unwrap());
    assert!(m.is_included("foo/bar/baz").unwrap());
    assert!(!m.is_included("foo/bar/qux").unwrap());
    assert!(m.is_included("qux").unwrap());
    assert!(m.is_included("qux/inner").unwrap());
    assert!(!m.is_included("other").unwrap());
    assert!(!m.is_included("qux/other").unwrap());
}

#[test]
fn files_from_directory_merges_rsync_filter() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("dir")).unwrap();
    fs::write(tmp.path().join("dir/.rsync-filter"), "- hidden\n").unwrap();
    fs::write(tmp.path().join("dir/hidden"), "h").unwrap();
    fs::write(tmp.path().join("dir/visible"), "v").unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "dir\n").unwrap();
    let filter = format!("files-from {}\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let m = Matcher::new(rules).with_root(tmp.path());
    assert!(m.is_included("dir/visible").unwrap());
    assert!(!m.is_included("dir/hidden").unwrap());
}
