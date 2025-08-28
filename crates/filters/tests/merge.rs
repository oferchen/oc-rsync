use filters::filters::{parse, Matcher};

#[test]
fn rsync_filter_merge() {
    let root_rules = parse("- *.tmp\n").unwrap();
    let mut matcher = Matcher::new(root_rules);

    assert!(matcher.is_included("notes.txt"));
    assert!(!matcher.is_included("junk.tmp"));

    // Merge rules from a subdirectory `.rsync-filter` file
    let sub_rules = parse("- secret\n").unwrap();
    matcher.merge(sub_rules);

    assert!(!matcher.is_included("junk.tmp"));
    assert!(!matcher.is_included("secret"));
}
