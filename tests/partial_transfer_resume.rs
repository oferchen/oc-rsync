// tests/partial_transfer_resume.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;
mod common;
use common::read_golden;

#[test]
fn partial_transfer_resumes_and_finishes() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("a.txt"), b"hello world").unwrap();
    fs::write(dst.join("a.partial"), &b"hello world"[..5]).unwrap();

    let src_arg = format!("{}/", src.display());
    let out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--partial")
        .arg(&src_arg)
        .arg(&dst)
        .output()
        .unwrap();

    let (exp_stdout, _exp_stderr, exp_exit) = read_golden("partial_transfer_resume");
    let output = String::from_utf8(out.stderr).unwrap();
    let filtered: String = output
        .lines()
        .filter(|l| *l != "recursive mode enabled")
        .collect::<Vec<_>>()
        .join("\n");

    assert_eq!(out.status.code(), Some(exp_exit));
    assert_eq!(filtered, String::from_utf8(exp_stdout).unwrap());
    assert_eq!(fs::read(dst.join("a.txt")).unwrap(), b"hello world");
}

#[test]
fn nested_partial_resume() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::create_dir_all(dst.join("sub")).unwrap();

    fs::write(src.join("sub/a.txt"), b"nested hello").unwrap();
    fs::write(dst.join("sub/a.partial"), &b"nested hello"[..6]).unwrap();

    let src_arg = format!("{}/", src.display());
    let out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--partial")
        .arg(&src_arg)
        .arg(&dst)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(fs::read(dst.join("sub/a.txt")).unwrap(), b"nested hello");
}
