// tests/log_file.rs
use assert_cmd::{Command as TestCommand, cargo::cargo_bin};
use std::{fs, process::Command as StdCommand};
use tempfile::tempdir;

struct LogFileTestBuilder {
    log_format: String,
}

impl LogFileTestBuilder {
    fn new() -> Self {
        Self {
            log_format: String::new(),
        }
    }

    fn log_format(mut self, fmt: &str) -> Self {
        self.log_format = fmt.to_owned();
        self
    }

    fn run(self) -> String {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("f"), b"hi").unwrap();
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&dst).unwrap();
        let log = tmp.path().join("log.txt");
        let src_arg = format!("{}/", src.display());
        let dst_arg = dst.to_str().unwrap();

        TestCommand::cargo_bin("oc-rsync")
            .unwrap()
            .arg("--log-file")
            .arg(&log)
            .arg(format!("--log-file-format={}", self.log_format))
            .arg("--out-format=%o %n")
            .arg(&src_arg)
            .arg(dst_arg)
            .assert()
            .success();

        fs::read_to_string(log).unwrap()
    }
}

#[test]
fn log_file_writes_messages() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("src"), b"hi").unwrap();
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&dst_dir).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "-v",
            &src_arg,
            dst_dir.to_str().unwrap(),
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
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&dst_dir).unwrap();
    let log = tmp.path().join("log.json");
    let src_arg = format!("{}/", src_dir.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=json",
            "-v",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("\"message\""), "{}", contents);
}

#[test]
fn log_file_format_token_t() {
    let contents = LogFileTestBuilder::new().log_format("%t").run();
    let line = contents.lines().find(|l| !l.starts_with("info::")).unwrap();
    let parts: Vec<_> = line.split_whitespace().collect();
    assert_eq!(parts.len(), 2);
    assert!(parts[0].contains('/'), "{}", line);
    assert!(parts[1].contains(':'), "{}", line);
}

#[test]
fn log_file_format_token_p() {
    let contents = LogFileTestBuilder::new().log_format("%p").run();
    let line = contents
        .lines()
        .find(|l| !l.starts_with("info::"))
        .unwrap()
        .trim();
    assert!(line.parse::<u32>().is_ok(), "{}", line);
}

#[test]
fn log_file_format_token_o() {
    let contents = LogFileTestBuilder::new().log_format("%o").run();
    let line = contents
        .lines()
        .find(|l| !l.starts_with("info::"))
        .unwrap()
        .trim();
    assert_eq!(line, "send");
}

#[test]
fn log_file_format_token_f() {
    let contents = LogFileTestBuilder::new().log_format("%f").run();
    let line = contents
        .lines()
        .find(|l| !l.starts_with("info::"))
        .unwrap()
        .trim();
    assert_eq!(line, "f");
}

#[test]
fn log_file_format_matches_rsync() {
    let tmp = tempdir().unwrap();
    let cwd = tmp.path();
    let src_dir = cwd.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    let dst_dir = cwd.join("dst");
    fs::create_dir_all(&dst_dir).unwrap();
    let log_oc = cwd.join("oc.log");
    let fmt = "\\t%o %f%i";
    let output = StdCommand::new(cargo_bin("oc-rsync"))
        .current_dir(cwd)
        .args([
            "--log-file",
            log_oc.to_str().unwrap(),
            &format!("--log-file-format={fmt}"),
            &format!("--out-format={fmt}"),
            "src/",
            "dst/",
        ])
        .output()
        .unwrap();

    let golden = "tests/golden/log_file_format/log_file_format_matches_rsync";
    let expected_exit: i32 = fs::read_to_string(format!("{golden}.exit"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(output.status.code().unwrap(), expected_exit);

    let ours = fs::read_to_string(&log_oc).unwrap();
    let ours_line = ours.lines().find(|l| l.contains(">f")).unwrap();
    let mut ours_line = ours_line.split_once("] ").unwrap().1.trim().to_string();
    if let Some(stripped) = ours_line.strip_suffix("send") {
        ours_line = stripped.trim_end().to_string();
    }
    let ours_line = format!("\\t{}", ours_line);
    let expected_line = fs::read_to_string(format!("{golden}.log")).unwrap();
    assert_eq!(ours_line, expected_line.trim());
}

#[test]
fn out_format_writes_custom_message() {
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("src"), b"hi").unwrap();
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&dst_dir).unwrap();
    let log = tmp.path().join("log.txt");
    let src_arg = format!("{}/", src_dir.display());
    TestCommand::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--log-file",
            log.to_str().unwrap(),
            "--out-format=custom:%n",
            &src_arg,
            dst_dir.to_str().unwrap(),
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
    assert!(contents.contains("send:f"), "{}", contents);
    assert!(contents.contains("ln -> f"), "{}", contents);
    assert!(contents.contains(">f"), "{}", contents);
    assert!(contents.contains('%'), "{}", contents);
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
    let ours_msg = ours
        .lines()
        .find(|l| !l.starts_with("info::"))
        .unwrap()
        .trim();

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
    let theirs_msg = theirs
        .lines()
        .find(|l| !l.is_empty() && !l.starts_with("info::"))
        .unwrap();

    assert_eq!(ours_msg, theirs_msg);
}
