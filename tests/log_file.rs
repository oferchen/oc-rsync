// tests/log_file.rs
use assert_cmd::{cargo::cargo_bin, Command as TestCommand};
use std::{fs, process::Command as StdCommand};
use tempfile::tempdir;

#[test]
#[ignore]
fn log_file_writes_messages() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    let dst_arg = dst.to_str().unwrap();
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "-v",
            src.to_str().unwrap(),
            dst_arg,
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("verbose level set to 1"), "{}", contents);
    assert!(!contents.contains("src"), "{}", contents);
}

#[test]
#[ignore]
fn log_file_format_json_writes_json() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.json");
    let dst_arg = dst.to_str().unwrap();
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=json",
            "-v",
            src.to_str().unwrap(),
            dst_arg,
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
    let dst_arg = dst.to_str().unwrap();
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=%t [%p] %o %f",
            "--out-format=%o %n",
            &src_arg,
            dst_arg,
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
    assert!(pid.parse::<u32>().is_ok());
    assert_eq!(op, "send");
    assert_eq!(file, "f");
}

#[test]
fn log_file_format_matches_rsync() {
    use logging::parse_escapes;

    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    let dst_oc = tmp.path().join("dst_oc");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&dst_oc).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let log_oc = tmp.path().join("oc.log");
    let log_rsync = tmp.path().join("rsync.log");
    let fmt = "\\t%o %f%i";
    let src_arg = format!("{}/", src_dir.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log_oc.to_str().unwrap(),
            &format!("--log-file-format={fmt}"),
            &format!("--out-format={fmt}"),
            &src_arg,
            dst_oc.to_str().unwrap(),
        ])
        .assert()
        .success();

    let fmt_rsync = parse_escapes(fmt);
    let output = StdCommand::new(cargo_bin("oc-rsync"))
        .args([
            "-r",
            &format!("--log-file={}", log_rsync.to_str().unwrap()),
            &format!("--log-file-format={}", fmt_rsync),
            &src_arg,
            dst_rsync.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let ours = fs::read_to_string(&log_oc).unwrap();
    let theirs = fs::read_to_string(&log_rsync).unwrap();
    let ours_line = ours.lines().find(|l| l.contains("send")).unwrap().trim();
    let theirs_line = theirs.lines().find(|l| l.contains("send")).unwrap().trim();
    assert_eq!(ours_line, theirs_line);
}

#[test]
#[ignore]
fn out_format_writes_custom_message() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    let dst_arg = dst.to_str().unwrap();
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "--out-format=custom:%n",
            src.to_str().unwrap(),
            dst_arg,
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("custom:src"), "{}", contents);
}

#[test]
#[cfg(unix)]
#[ignore]
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
    let dst_arg = format!("{}/", dst.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-l",
            "--log-file",
            log.to_str().unwrap(),
            &format!("--out-format={fmt}"),
            &src_arg,
            dst_arg.as_str(),
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
#[ignore]
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

    let dst_arg = format!("{}/", dst_oc.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            &format!("--out-format={fmt}"),
            &src_arg,
            dst_arg.as_str(),
        ])
        .assert()
        .success();
    let ours = fs::read_to_string(&log).unwrap();
    let ours_line = ours
        .lines()
        .find(|l| l.contains("info::name") && l.contains("send"))
        .unwrap();
    let ours_msg = ours_line.split("info::name: ").nth(1).unwrap();

    let output = StdCommand::new(cargo_bin("oc-rsync"))
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
