// crates/filters/tests/posix_classes_golden.rs
use filters::{Matcher, parse};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fn check(class: &str, cases: &[(&str, bool)]) {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root).unwrap();
    for (name, _) in cases {
        fs::write(root.join(name), "").unwrap();
    }
    let rules_src = format!("+ file[[:{}:]]name\n- *\n", class);
    let mut visited = HashSet::new();
    let rules = parse(&rules_src, &mut visited, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);
    for (name, expected) in cases {
        assert_eq!(
            matcher.is_included(name).unwrap(),
            *expected,
            "class {class} file {name:?}"
        );
    }
}

#[test]
fn space_class() {
    check(
        "space",
        &[
            ("file name", true),
            ("file\tname", true),
            ("file\nname", true),
            ("filename", false),
        ],
    );
}

#[test]
fn blank_class() {
    check(
        "blank",
        &[
            ("file name", true),
            ("file\tname", true),
            ("file\nname", false),
            ("filename", false),
        ],
    );
}

#[test]
fn punct_class() {
    check(
        "punct",
        &[
            ("file-name", true),
            ("file.name", true),
            ("filename", false),
            ("file name", false),
        ],
    );
}

#[test]
fn cntrl_class() {
    check("cntrl", &[("file\u{1}name", true), ("filename", false)]);
}

#[test]
fn graph_class() {
    check(
        "graph",
        &[
            ("file-name", true),
            ("file1name", true),
            ("file name", false),
        ],
    );
}

#[test]
fn print_class() {
    check(
        "print",
        &[
            ("file-name", true),
            ("file name", true),
            ("file\u{1}name", false),
        ],
    );
}
