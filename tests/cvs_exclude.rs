// tests/cvs_exclude.rs

use assert_cmd::Command;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;

#[test]
fn cvs_exclude_parity() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(src.join(".git")).unwrap();
    fs::write(src.join(".git/file"), "git").unwrap();
    fs::write(src.join("keep.txt"), "keep\n").unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo.o"), "obj").unwrap();
    fs::write(src.join("env_ignored"), "env").unwrap();
    fs::write(src.join("home_ignored"), "home").unwrap();
    fs::write(src.join("local_ignored"), "local").unwrap();
    fs::write(src.join(".cvsignore"), "local_ignored\n").unwrap();

    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("local_ignored"), "sublocal\n").unwrap();
    fs::write(sub.join("env_ignored"), "env").unwrap();
    fs::write(sub.join("home_ignored"), "home").unwrap();
    fs::write(sub.join("sub_ignored"), "sub").unwrap();
    fs::write(sub.join(".cvsignore"), "sub_ignored\n").unwrap();

    let nested = sub.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("sub_ignored"), "nested\n").unwrap();

    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();

    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&ours_dst).unwrap();

    let src_arg = format!("{}/", src.display());

    let mut ours_cmd = Command::cargo_bin("oc-rsync").unwrap();
    ours_cmd.env("CVSIGNORE", "env_ignored");
    ours_cmd.env("HOME", home.path());
    ours_cmd.args(["--recursive", "--cvs-exclude"]);
    ours_cmd.arg(&src_arg);
    ours_cmd.arg(&ours_dst);
    let ours_out = ours_cmd.output().unwrap();
    assert!(ours_out.status.success());
    let mut ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    ours_output = ours_output.replace("recursive mode enabled\n", "");

    assert!(ours_output.is_empty());

    assert_dirs_equal(Path::new("tests/golden/cvs_exclude/expected"), &ours_dst);
}

#[test]
fn cvs_exclude_nested_override() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("keep.txt"), "keep\n").unwrap();
    fs::write(src.join("core"), "core\n").unwrap();

    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".cvsignore"), "nested/\n").unwrap();

    let nested = sub.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("keep.txt"), "keep\n").unwrap();
    fs::write(nested.join("core"), "core\n").unwrap();

    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let src_arg = format!("{}/", src.display());

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args([
        "--recursive",
        "--cvs-exclude",
        "--include=sub/nested/",
        "--include=sub/nested/***",
    ]);
    cmd.arg(&src_arg);
    cmd.arg(&dst);
    let out = cmd.output().unwrap();
    assert!(out.status.success());

    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("core").exists());
    assert!(dst.join("sub/nested/keep.txt").exists());
    assert!(dst.join("sub/nested/core").exists());
}

fn assert_dirs_equal(expected: &Path, actual: &Path) {
    fn collect(root: &Path) -> (HashSet<PathBuf>, HashSet<PathBuf>) {
        let mut files = HashSet::new();
        let mut dirs = HashSet::new();
        for entry in WalkDir::new(root).min_depth(1) {
            let entry = entry.unwrap();
            let rel = entry.path().strip_prefix(root).unwrap().to_path_buf();
            if entry.file_type().is_dir() {
                dirs.insert(rel);
            } else if entry.file_type().is_file() {
                files.insert(rel);
            }
        }
        (files, dirs)
    }

    let (expected_files, expected_dirs) = collect(expected);
    let (actual_files, actual_dirs) = collect(actual);

    let missing_dirs: Vec<_> = expected_dirs.difference(&actual_dirs).cloned().collect();
    let extra_dirs: Vec<_> = actual_dirs.difference(&expected_dirs).cloned().collect();
    let missing_files: Vec<_> = expected_files.difference(&actual_files).cloned().collect();
    let extra_files: Vec<_> = actual_files.difference(&expected_files).cloned().collect();

    assert!(
        missing_dirs.is_empty()
            && extra_dirs.is_empty()
            && missing_files.is_empty()
            && extra_files.is_empty(),
        "directory trees differ\nmissing_dirs: {:?}\nextra_dirs: {:?}\nmissing_files: {:?}\nextra_files: {:?}",
        missing_dirs,
        extra_dirs,
        missing_files,
        extra_files
    );

    for rel in expected_files {
        let exp_data = fs::read(expected.join(&rel)).unwrap();
        let act_data = fs::read(actual.join(&rel)).unwrap();
        assert_eq!(exp_data, act_data, "file contents differ for {:?}", rel);
    }
}
