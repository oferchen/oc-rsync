// tests/files_from_dirs.rs
use filters::{Matcher, parse_with_options};
use std::collections::HashSet;
use std::fs;
use std::process::Command;
use tempfile::tempdir;
use walk::walk;
mod util;
use util::setup_files_from_env;

#[test]
fn files_from_mixed_entries_integration() {
    let tmp = tempdir().unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz\nqux/\n").unwrap();
    let filter = format!("files-from {}\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let m = Matcher::new(rules);
    let foo = m.is_included_with_dir("foo").unwrap();
    assert!(foo.include && foo.descend);
    let foo_bar = m.is_included_with_dir("foo/bar").unwrap();
    assert!(foo_bar.include && foo_bar.descend);
    assert!(m.is_included("foo/bar/baz").unwrap());
    assert!(!m.is_included("foo/bar/qux").unwrap());
    assert!(!m.is_included("foo/other").unwrap());
    let qux = m.is_included_with_dir("qux").unwrap();
    assert!(qux.include && qux.descend);
    let qux_sub = m.is_included_with_dir("qux/sub").unwrap();
    assert!(qux_sub.include && qux_sub.descend);
    assert!(!m.is_included("other").unwrap());
    assert!(!m.is_included("qux/other").unwrap());
}

#[test]
fn walker_files_from_enumerates_parent_dirs() {
    let (tmp, src, _) = setup_files_from_env(&[("foo/bar/baz", b"data")]);

    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz\n").unwrap();
    let filter = format!("files-from {}\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let matcher = Matcher::new(rules);

    let mut walker = walk(&src, 1, None, false, false, &[]).unwrap();
    let mut state = String::new();
    let mut visited = Vec::new();
    while let Some(batch) = walker.next() {
        let batch = batch.unwrap();
        for entry in batch {
            let path = entry.apply(&mut state);
            let rel = path
                .strip_prefix(&src)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel.is_empty() {
                continue;
            }
            let result = matcher.is_included_with_dir(&rel).unwrap();
            if result.include {
                if entry.file_type.is_dir() {
                    assert!(result.descend);
                    visited.push(format!("{rel}/"));
                } else {
                    visited.push(rel.clone());
                }
            }
            if entry.file_type.is_dir() && !result.descend {
                walker.skip_current_dir();
            }
        }
    }

    let output = Command::new("rsync")
        .current_dir(&src)
        .args([
            "-n",
            "-r",
            "--out-format=%n",
            "--files-from",
            list.to_str().unwrap(),
            ".",
            "dest",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let rsync_paths: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    assert_eq!(visited, rsync_paths);
    assert_eq!(visited, vec!["foo/", "foo/bar/", "foo/bar/baz"]);
}
