// tests/files_from_dirs.rs
use assert_cmd::prelude::*;
use assert_cmd::Command;
use filters::{parse_with_options, Matcher};
use std::collections::HashSet;
use std::fs;
use std::process::Command as StdCommand;
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
    let m = Matcher::new(rules.clone());
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

    let m_no = Matcher::new(rules).with_no_implied_dirs();
    let foo = m_no.is_included_with_dir("foo").unwrap();
    assert!(!foo.include && foo.descend);
    let foo_bar = m_no.is_included_with_dir("foo/bar").unwrap();
    assert!(!foo_bar.include && foo_bar.descend);
    assert!(m_no.is_included("foo/bar/baz").unwrap());
    let qux = m_no.is_included_with_dir("qux").unwrap();
    assert!(qux.include && qux.descend);
    let qux_sub = m_no.is_included_with_dir("qux/sub").unwrap();
    assert!(qux_sub.include && qux_sub.descend);
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

    let output = StdCommand::new("rsync")
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

#[test]
fn files_from_dirs_matches_rsync() {
    let (tmp, src, _) =
        setup_files_from_env(&[("foo/bar/baz", b"data"), ("qux/sub/keep.txt", b"data")]);

    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz\nqux/\n").unwrap();

    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    let status = StdCommand::new("rsync")
        .current_dir(&src)
        .args([
            "-r",
            "--files-from",
            list.to_str().unwrap(),
            ".",
            rsync_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(&src)
        .args([
            "--recursive",
            "--files-from",
            list.to_str().unwrap(),
            ".",
            ours_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .status()
        .unwrap();
    assert!(diff.success(), "directory trees differ");
}
