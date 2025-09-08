// tests/stats.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn stats_parity() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_ours = dir.path().join("dst_ours");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.txt"), b"hello").unwrap();

    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/stats/stats_parity.stdout");
    let up_stats: Vec<String> = fs::read_to_string(golden)
        .unwrap()
        .lines()
        .map(|l| l.to_string())
        .collect();
    assert_eq!(up_stats.len(), 6);
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--recursive",
            "--stats",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        ours.status.success(),
        "oc-rsync failed: {}",
        String::from_utf8_lossy(&ours.stderr)
    );

    let our_stdout = String::from_utf8_lossy(&ours.stdout);
    let our_stats: Vec<String> = our_stdout
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            if l.starts_with("Number of files")
                || l.starts_with("Number of created files")
                || l.starts_with("Number of deleted files")
                || l.starts_with("Number of regular files transferred")
                || l.starts_with("Total transferred file size")
                || l.starts_with("File list size")
            {
                Some(l.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(our_stats, up_stats);

    let rate_line = our_stdout
        .lines()
        .find_map(|l| {
            let l = l.trim_start();
            l.starts_with("sent ").then(|| l.to_string())
        })
        .expect("missing rate line");
    assert!(rate_line.contains("KB/s"));

    insta::assert_snapshot!("stats_parity", our_stats.join("\n"));
}

#[test]
fn stats_are_printed() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("a.txt"), b"hello").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--stats", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success().stdout(predicates::str::contains(
        "Number of regular files transferred",
    ));
}
