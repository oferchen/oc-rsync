// tests/skip_compress.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn skip_compress_option_transfers_files() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.gz"), b"data").unwrap();
    fs::write(src.join("b.txt"), b"text").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--compress",
            "--skip-compress=gz,txt",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let out_gz = fs::read(dst.join("a.gz")).unwrap();
    assert_eq!(out_gz, b"data");
    let out_txt = fs::read(dst.join("b.txt")).unwrap();
    assert_eq!(out_txt, b"text");
}
