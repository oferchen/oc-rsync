// tests/log_file.rs
use assert_cmd::Command as TestCommand;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn log_file_writes_messages() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "-v",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("verbose level set to 1"), "{}", contents);
    assert!(!contents.contains("src"), "{}", contents);
}

#[test]
fn log_file_format_json_writes_json() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.json");
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=json",
            "-v",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("\"message\""), "{}", contents);
}

#[test]
fn log_file_format_tokens() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=%t [%p] %o %f",
            "--out-format=%o %n",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    let line = contents.lines().find(|l| l.contains("send")).unwrap();
    let mut parts = line.split_whitespace();
    let date = parts.next().unwrap();
    let time = parts.next().unwrap();
    let pid = parts.next().unwrap().trim_matches(|c| c == '[' || c == ']');
    let op = parts.next().unwrap();
    let file = parts.next().unwrap();
    assert!(date.contains('/'));
    assert!(time.contains(':'));
    assert_eq!(pid, std::process::id().to_string());
    assert_eq!(op, "send");
    assert_eq!(file, "f");
}

#[test]
fn out_format_writes_custom_message() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "--out-format=custom:%n",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("custom:src"), "{}", contents);
}

#[test]
#[cfg(unix)]
fn out_format_supports_all_escapes() {
    use std::os::unix::fs::symlink;

    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    symlink("f", src_dir.join("ln")).unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    let log = tmp.path().join("log.txt");
    let fmt = "\t%o:%n%L%i%%\\\n";
    let src_arg = format!("{}/", src_dir.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "-l",
            "--log-file",
            log.to_str().unwrap(),
            &format!("--out-format={fmt}"),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("\tsend:"), "{}", contents);
    assert!(contents.contains("ln -> f"), "{}", contents);
    assert!(contents.contains(">f"), "{}", contents);
    assert!(contents.contains("%\\\n"), "{}", contents);
}
#[test]
fn out_format_escapes_match_rsync() {
    use logging::parse_escapes;

    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&dst_oc).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let log = tmp.path().join("log.txt");
    let fmt = "\\t%o:%n\\x21";
    let fmt_rsync = parse_escapes(fmt);
    let src_arg = format!("{}/", src_dir.display());

    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            &format!("--out-format={fmt}"),
            &src_arg,
            dst_oc.to_str().unwrap(),
        ])
        .assert()
        .success();
    let ours = fs::read_to_string(&log).unwrap();
    let ours_line = ours
        .lines()
        .find(|l| l.contains("info::name") && l.contains("send"))
        .unwrap();
    let ours_msg = ours_line.split("info::name: ").nth(1).unwrap();

    let output = Command::new("rsync")
        .args([
            "-r",
            &format!("--out-format={fmt_rsync}"),
            &src_arg,
            dst_rsync.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let theirs = String::from_utf8_lossy(&output.stdout);
    let theirs_msg = theirs.lines().find(|l| !l.is_empty()).unwrap();

    assert_eq!(ours_msg, theirs_msg);
}
