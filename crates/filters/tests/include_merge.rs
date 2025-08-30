use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn nested_include_merge_applies() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Create nested filter files
    let rules3 = root.join("rules3");
    fs::write(&rules3, "- tmp/\n").unwrap();

    let rules2 = root.join("rules2");
    fs::write(
        &rules2,
        format!(":include-merge {}\n+ *.txt\n- *\n", rules3.display()),
    )
    .unwrap();

    let rules1 = root.join("rules1");
    fs::write(&rules1, format!(":include-merge {}\n", rules2.display())).unwrap();

    // Source tree
    fs::create_dir_all(root.join("src/dir")).unwrap();
    fs::create_dir_all(root.join("src/tmp")).unwrap();
    fs::write(root.join("src/root.txt"), "root").unwrap();
    fs::write(root.join("src/dir/file.txt"), "file").unwrap();
    fs::write(root.join("src/tmp/file.txt"), "tmp").unwrap();

    let mut v = HashSet::new();
    let rules = parse(&format!(":include-merge {}\n", rules1.display()), &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root.join("src"));

    assert!(!matcher.is_included("tmp/file.txt").unwrap());
    assert!(matcher.is_included("root.txt").unwrap());
    assert!(matcher.is_included("dir/file.txt").unwrap());
}
