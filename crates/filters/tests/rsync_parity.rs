use filters::{parse, Matcher};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn parity_with_stock_rsync() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("src");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join(".rsync-filter"), "- *.tmp\n").unwrap();
    fs::write(root.join("a.tmp"), "").unwrap();
    fs::write(root.join("keep.log"), "").unwrap();
    fs::write(root.join("sub/.rsync-filter"), "+ keep.tmp\n").unwrap();
    fs::write(root.join("sub/keep.tmp"), "").unwrap();
    fs::write(root.join("sub/other.tmp"), "").unwrap();

    let rules = parse(": /.rsync-filter\n- .rsync-filter\n").unwrap();
    let matcher = Matcher::new(rules).with_root(&root);

    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    let root_arg = format!("{}/", root.display());
    let output = Command::new("rsync")
        .args(["-r", "-n", "-i", "-FF", &root_arg, dest.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut rsync_included = Vec::new();
    for line in stdout.lines() {
        if line.starts_with("sending ") || line.starts_with("sent ") || line.starts_with("total ") {
            continue;
        }
        if let Some(name) = line.split_whitespace().last() {
            rsync_included.push(name.to_string());
        }
    }

    let files = [
        "a.tmp",
        "keep.log",
        "sub/keep.tmp",
        "sub/other.tmp",
        ".rsync-filter",
        "sub/.rsync-filter",
    ];
    for f in files {
        let ours = matcher.is_included(f).unwrap();
        let theirs = rsync_included.contains(&f.to_string());
        assert_eq!(ours, theirs, "file {}", f);
    }
}
