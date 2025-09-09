// tests/cvs_exclude.rs

use assert_cmd::Command;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;

#[test]
fn cvs_exclude_default_rules() {
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

    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();

    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let src_arg = format!("{}/", src.display());

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.env("CVSIGNORE", "env_ignored");
    cmd.env("HOME", home.path());
    cmd.args(["--recursive", "--cvs-exclude"]);
    cmd.arg(&src_arg);
    cmd.arg(&dst);
    assert!(cmd.output().unwrap().status.success());

    assert!(
        dst.join("keep.txt").exists(),
        "regular files should be included"
    );
    assert!(
        dst.join(".cvsignore").exists(),
        ".cvsignore files should be transferred"
    );
    assert!(
        !dst.join(".git").exists(),
        ".git directory should be excluded by default"
    );
    assert!(
        !dst.join("core").exists(),
        "'core' files should be excluded by default"
    );
    assert!(
        !dst.join("foo.o").exists(),
        "object files (*.o) should be excluded by default",
    );
    assert!(
        !dst.join("env_ignored").exists(),
        "CVSIGNORE environment variable should exclude env_ignored",
    );
    assert!(
        !dst.join("home_ignored").exists(),
        "HOME/.cvsignore should exclude home_ignored",
    );
    assert!(
        !dst.join("local_ignored").exists(),
        "local .cvsignore should exclude local_ignored",
    );
}

#[test]
fn include_overrides_cvs_exclude_on_nested_paths() {
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
    assert!(cmd.output().unwrap().status.success());

    assert!(
        dst.join("keep.txt").exists(),
        "baseline file should transfer"
    );
    assert!(
        !dst.join("core").exists(),
        "'core' should be excluded outside of the include rule",
    );
    assert!(
        dst.join("sub/nested/keep.txt").exists(),
        "--include should allow keep.txt in the nested directory",
    );
    assert!(
        dst.join("sub/nested/core").exists(),
        "--include should override exclusion for core in the nested directory",
    );
}

#[test]
fn cvsignore_is_scoped_per_directory() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join(".cvsignore"), "foo\n").unwrap();
    fs::write(src.join("foo"), b"root").unwrap();
    fs::write(src.join("bar"), b"rootbar").unwrap();

    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".cvsignore"), "bar\n").unwrap();
    fs::write(sub.join("foo"), b"subfoo").unwrap();
    fs::write(sub.join("bar"), b"subbar").unwrap();
    fs::create_dir_all(sub.join("nested")).unwrap();
    fs::write(sub.join("nested/bar"), b"nestedbar").unwrap();

    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--cvs-exclude",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        !dst.join("foo").exists(),
        "root .cvsignore should exclude foo",
    );
    assert!(
        dst.join("bar").exists(),
        "bar should not be excluded at root"
    );
    assert!(
        dst.join("sub/foo").exists(),
        "root .cvsignore should not apply to sub/foo",
    );
    assert!(
        !dst.join("sub/bar").exists(),
        "sub/.cvsignore should exclude bar in sub directory",
    );
    assert!(
        dst.join("sub/nested/bar").exists(),
        "sub/.cvsignore should not apply to sub/nested",
    );
    assert!(
        dst.join(".cvsignore").exists(),
        ".cvsignore files themselves should be transferred",
    );
}

#[test]
fn filter_c_equivalent_to_filters() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("local_ignored"), "local").unwrap();
    fs::write(src.join(".cvsignore"), "local_ignored\n").unwrap();

    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();
    fs::write(src.join("home_ignored"), "home").unwrap();
    fs::write(src.join("env_ignored"), "env").unwrap();

    let src_arg = format!("{}/", src.display());

    let dst1 = tmp.path().join("dst1");
    fs::create_dir_all(&dst1).unwrap();
    let mut cmd1 = Command::cargo_bin("oc-rsync").unwrap();
    cmd1.env("CVSIGNORE", "env_ignored");
    cmd1.env("HOME", home.path());
    cmd1.args(["--recursive", "--cvs-exclude"]);
    cmd1.arg(&src_arg);
    cmd1.arg(&dst1);
    assert!(cmd1.output().unwrap().status.success());

    let dst2 = tmp.path().join("dst2");
    fs::create_dir_all(&dst2).unwrap();
    let mut cmd2 = Command::cargo_bin("oc-rsync").unwrap();
    cmd2.env("CVSIGNORE", "env_ignored");
    cmd2.env("HOME", home.path());
    cmd2.args(["--recursive", "--filter=-C", "--filter=:C"]);
    cmd2.arg(&src_arg);
    cmd2.arg(&dst2);
    assert!(cmd2.output().unwrap().status.success());

    assert_dirs_equal(&dst1, &dst2);
}

#[test]
fn filter_minus_c_ignores_defaults_only() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo"), "foo").unwrap();
    fs::write(src.join(".cvsignore"), "foo\n").unwrap();

    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--recursive", "--filter=-C"]);
    cmd.arg(&src_arg);
    cmd.arg(&dst);
    assert!(cmd.output().unwrap().status.success());

    assert!(dst.join("foo").exists());
    assert!(!dst.join("core").exists());
}

#[test]
fn filter_colon_c_uses_local_cvsignore_only() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo"), "foo").unwrap();
    fs::write(src.join(".cvsignore"), "foo\n").unwrap();

    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let src_arg = format!("{}/", src.display());
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.args(["--recursive", "--filter=:C"]);
    cmd.arg(&src_arg);
    cmd.arg(&dst);
    assert!(cmd.output().unwrap().status.success());

    assert!(!dst.join("foo").exists());
    assert!(dst.join("core").exists());
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
