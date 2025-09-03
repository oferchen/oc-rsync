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
    let expected: Vec<_> = include_str!("fixtures/dry_run_deletions.txt")
        .lines()
        .collect();
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
    assert_eq!(expected, our_lines);
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
    let fixture = include_str!("fixtures/dry_run_errors.stderr");
    let path = tmp.path().join("missing.txt");
    let expected = fixture.replace("{path}", path.to_str().unwrap());
    let ours_err = String::from_utf8(ours.stderr).unwrap();
    let mut expected_lines = expected.lines();
    let mut our_lines = ours_err.lines();
    assert_eq!(expected_lines.next(), our_lines.next());
    assert_eq!(expected_lines.next(), our_lines.next());
}
