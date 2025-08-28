use filters::{parse, Matcher};

#[test]
fn root_anchored_exclusion() {
    let rules = parse("- /root.txt\n+ *.txt\n- *\n").unwrap();
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("root.txt").unwrap());
    assert!(matcher.is_included("dir/root.txt").unwrap());
}

#[test]
fn directory_trailing_slash() {
    let rules = parse("- tmp/\n").unwrap();
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("tmp").unwrap());
    assert!(!matcher.is_included("tmp/file.txt").unwrap());
    assert!(matcher.is_included("other/file.txt").unwrap());
}

#[test]
fn wildcard_question_mark() {
    let rules = parse("+ file?.txt\n- *\n").unwrap();
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("file1.txt").unwrap());
    assert!(!matcher.is_included("file10.txt").unwrap());
}

#[test]
fn double_star_matches() {
    let rules = parse("+ dir/**/keep.txt\n- *\n").unwrap();
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("dir/a/b/keep.txt").unwrap());
    assert!(!matcher.is_included("dir/a/b/drop.txt").unwrap());
}
