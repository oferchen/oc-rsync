// crates/filters/tests/from0_merges.rs
use filters::{parse, parse_list, parse_with_options, Matcher};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn list_parsing_from0_eq_newline(entries in prop::collection::vec("[^\0#\r\n]*", 0..4)) {
        let mut bytes = Vec::new();
        for e in &entries {
            if !e.is_empty() {
                bytes.extend(e.as_bytes());
            }
            bytes.push(0);
        }
        let parsed0 = parse_list(&bytes, true);
        let joined = entries.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
        let parsed_nl = parse_list(joined.as_bytes(), false);
        prop_assert_eq!(parsed0, parsed_nl);
    }
}

#[test]
fn merge_word_split_from0() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let list = root.join("rules");
    fs::write(&list, b"-foo\0+bar\0").unwrap();
    let spec = format!(": merge,w {}\n", list.display());
    let mut v: HashSet<PathBuf> = HashSet::new();
    let rules = parse_with_options(&spec, true, &mut v, 0, None).unwrap();
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
}

proptest! {
    #[test]
    fn recursive_merge_excludes_file(_dummy in any::<bool>()) {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(".rsync-filter"), ": filter\n- .rsync-filter\n").unwrap();
        fs::write(root.join("filter"), "- foo\n").unwrap();

        let mut v = HashSet::new();
        let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
        let matcher = Matcher::new(rules).with_root(root);
        prop_assert!(!matcher.is_included("foo").unwrap());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn parse_list_ignores_empty(entries in prop::collection::vec("[a-z]{0,3}", 0..4)) {
        let mut bytes = Vec::new();
        for e in &entries {
            bytes.extend(e.as_bytes());
            bytes.push(0);
        }
        bytes.extend(&[0,0]);
        let parsed = parse_list(&bytes, true);
        let expected: Vec<String> = entries.into_iter().filter(|s| !s.is_empty()).collect();
        prop_assert_eq!(parsed, expected);
    }
}
