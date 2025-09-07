// crates/cli/tests/progress_stats.rs
use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn progress_parity() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--progress",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let norm = |s: &[u8]| {
        let txt = String::from_utf8_lossy(s).replace('\r', "\n");
        txt.lines()
            .rev()
            .find(|l| l.contains('%'))
            .and_then(|l| l.split(" (xfr").next())
            .unwrap()
            .to_string()
    };
    let our_line = norm(&ours.stderr);
    let our_parts: Vec<_> = our_line.split_whitespace().collect();
    assert_eq!(our_parts.first(), Some(&"5"));
    assert_eq!(our_parts.get(1), Some(&"100%"));
    assert!(our_parts.get(2).is_some_and(|s| s.ends_with("KB/s")));
    let rate_placeholder: String = our_parts[2]
        .chars()
        .map(|c| if c.is_ascii_digit() { 'X' } else { c })
        .collect();
    let normalized = format!(
        "{:>15} {:>4} {} {}",
        our_parts[0], our_parts[1], rate_placeholder, our_parts[3]
    );
    insta::assert_snapshot!("progress_parity", normalized);
}

#[test]
fn stats_parity() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_ours = dir.path().join("dst_ours");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), b"hello").unwrap();
    let ours = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("COLUMNS", "80")
        .args([
            "--stats",
            format!("{}/", src.display()).as_str(),
            dst_ours.to_str().unwrap(),
        ])
        .output()
        .unwrap();

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

    let expected = [
        "Number of files: 1",
        "Number of created files: 1",
        "Number of deleted files: 0",
        "Number of regular files transferred: 1",
        "Total transferred file size: 5 bytes",
        "File list size: 0",
    ];
    assert_eq!(our_stats, expected);
    insta::assert_snapshot!("stats_parity", our_stats.join("\n"));
}
