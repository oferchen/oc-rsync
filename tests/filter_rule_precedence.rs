// tests/filter_rule_precedence.rs
use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use walkdir::WalkDir;
mod common;
use common::read_golden;

fn collect(dir: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for entry in WalkDir::new(dir) {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let rel = entry
                .path()
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let contents = fs::read_to_string(entry.path()).unwrap();
            map.insert(rel, contents);
        }
    }
    map
}

#[test]
fn filter_rule_precedence() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("keep/sub")).unwrap();
    fs::create_dir_all(src.join("keep/tmp")).unwrap();
    fs::create_dir_all(src.join("skip")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    fs::write(src.join("keep/file.txt"), "keep").unwrap();
    fs::write(src.join("keep/sub/file.md"), "sub").unwrap();
    fs::write(src.join("keep/tmp/file.tmp"), "tmp").unwrap();
    fs::write(src.join("skip/file.txt"), "skip").unwrap();
    fs::write(src.join("root.tmp"), "root").unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo.o"), "obj").unwrap();
    fs::write(src.join("debug.log"), "debug").unwrap();
    fs::write(src.join("info.log"), "info").unwrap();
    fs::write(src.join("keep/info.log"), "info").unwrap();

    fs::write(src.join(".rsync-filter"), "+ keep/tmp/file.tmp\n- *.tmp\n").unwrap();
    fs::write(src.join(".gitignore"), "- *.log\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let out = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--recursive")
        .args([
            "--filter=: /.rsync-filter",
            "--filter=: /.gitignore",
            "--filter=- .rsync-filter",
            "--filter=- .gitignore",
            "--filter=+ core",
            "--filter=-C",
            "--filter=S debug.log",
            "--filter=- skip/",
            "--filter=+ keep/***",
            "--filter=+ *.md",
            "--filter=- *",
        ])
        .arg(&src_arg)
        .arg(&dst)
        .output()
        .unwrap();

    let (exp_stdout, _exp_stderr, exp_exit) = read_golden("filter_rule_precedence");
    let combined = String::from_utf8(out.stderr).unwrap();
    let filtered: String = combined
        .lines()
        .filter(|l| *l != "recursive mode enabled")
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(out.status.code(), Some(exp_exit));
    assert_eq!(filtered, String::from_utf8(exp_stdout).unwrap());

    let expected = collect(Path::new("tests/golden/filter_rule_precedence/dst"));
    let actual = collect(&dst);
    assert_eq!(actual, expected);
}
