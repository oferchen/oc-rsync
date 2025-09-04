// tests/dry_run.rs

use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn dry_run_deletions_match_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(dst.join("old.txt"), b"old").unwrap();

    let src_arg = format!("{}/", src.display());
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--delete",
            "--dry-run",
            "--verbose",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let our_out = String::from_utf8(ours.stdout).unwrap();
    let our_lines: Vec<_> = our_out
        .lines()
        .filter(|l| l.starts_with("deleting "))
        .collect();

    let rsync_out = fs::read_to_string("tests/golden/dry_run/deletions.txt").unwrap();
    let rsync_lines: Vec<_> = rsync_out.lines().collect();

    assert_eq!(rsync_lines, our_lines);
}

#[test]
fn dry_run_errors_match_rsync() {
    let tmp = tempdir().unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();

    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .current_dir(tmp.path())
        .args(["--dry-run", "missing.txt", dst.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(ours.status.code(), Some(23));
    let ours_err = String::from_utf8(ours.stderr).unwrap();

    let expected = fs::read_to_string("tests/golden/dry_run/error.txt").unwrap();
    let mut expected_lines = expected.lines();
    let mut our_lines = ours_err.lines();

    let first = expected_lines.next().unwrap().replace(
        "{PATH}",
        &tmp.path().join("missing.txt").display().to_string(),
    );
    assert_eq!(Some(first.as_str()), our_lines.next());

    let exp_prefix = expected_lines.next().unwrap();
    let our_second = our_lines.next().unwrap();
    let our_prefix = our_second.split(" at ").next().unwrap();
    assert_eq!(exp_prefix, our_prefix);
}
