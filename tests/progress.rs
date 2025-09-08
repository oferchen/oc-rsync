// tests/progress.rs
use assert_cmd::Command;
use assert_cmd::prelude::*;
use logging::progress_formatter;
use std::fs;
use tempfile::tempdir;

#[test]
fn progress_flag_shows_output() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), vec![0u8; 2048]).unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    let assert = cmd
        .args([
            "--recursive",
            "--progress",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(stderr.is_empty(), "{}", stderr);
    let mut lines = stdout.lines();
    assert_eq!(lines.next().unwrap(), "sending incremental file list");
    assert_eq!(lines.next().unwrap(), "a.txt");
    let progress_line_raw = lines.next().unwrap();
    let progress_line = progress_line_raw.trim_start_matches('\r').trim_end();
    let bytes = progress_formatter(2048, false);
    let expected_prefix = format!("{:>15} {:>3}%", bytes, 100);
    assert!(progress_line.starts_with(&expected_prefix));
    assert!(stdout.contains(progress_line_raw));
}

fn normalize_progress_line(line: &str) -> String {
    let mut parts: Vec<_> = line.split_whitespace().collect();
    if parts.len() >= 4 {
        parts[2] = "XKB/s";
        parts[3] = "00:00:00";
        format!("{:>15} {:>4} {} {}", parts[0], parts[1], parts[2], parts[3])
    } else {
        line.to_string()
    }
}

fn progress_parity_impl(flags: &[&str], fixture: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst_ours = dir.path().join("dst_ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst_ours).unwrap();
    fs::write(src.join("a.txt"), b"hello").unwrap();

    let mut our_cmd = Command::cargo_bin("oc-rsync").unwrap();
    our_cmd.env("LC_ALL", "C").env("COLUMNS", "80");
    our_cmd.args(flags);
    our_cmd.arg(format!("{}/", src.display()));
    our_cmd.arg(dst_ours.to_str().unwrap());
    let ours = our_cmd.output().unwrap();

    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/progress")
        .join(fixture);
    let up_stdout = fs::read(golden.with_extension("stdout")).unwrap();
    let up_stderr = fs::read(golden.with_extension("stderr")).unwrap();
    let up_status: i32 = fs::read_to_string(golden.with_extension("exit"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    assert_eq!(Some(up_status), ours.status.code(), "exit status mismatch");

    (up_stdout, up_stderr, ours.stdout, ours.stderr)
}

fn extract_progress_line(stdout: &[u8], stderr: &[u8]) -> (String, String, String, &'static str) {
    let stdout_txt = String::from_utf8_lossy(stdout).replace('\r', "\n");
    let stderr_txt = String::from_utf8_lossy(stderr).replace('\r', "\n");
    let find = |txt: &str| {
        txt.lines()
            .rev()
            .find(|l| l.contains('%'))
            .map(|l| l.to_string())
    };
    if let Some(line) = find(&stdout_txt) {
        (line, stdout_txt, stderr_txt, "stdout")
    } else if let Some(line) = find(&stderr_txt) {
        (line, stdout_txt, stderr_txt, "stderr")
    } else {
        panic!("no progress line found");
    }
}

fn assert_progress_stream(expected: &str, actual: &str) {
    assert_eq!(expected, actual, "progress output stream mismatch");
}

fn assert_non_progress_output(
    up_stdout_txt: &str,
    up_stderr_txt: &str,
    our_stdout_txt: &str,
    our_stderr_txt: &str,
) {
    fn strip_progress(s: &str) -> String {
        s.lines()
            .filter(|l| !l.contains('%'))
            .collect::<Vec<_>>()
            .join("\n")
    }
    assert_eq!(
        strip_progress(up_stdout_txt),
        strip_progress(our_stdout_txt),
        "stdout mismatch without progress lines",
    );
    assert_eq!(
        strip_progress(up_stderr_txt),
        strip_progress(our_stderr_txt),
        "stderr mismatch without progress lines",
    );
}

#[test]
fn progress_parity() {
    let (up_stdout, up_stderr, our_stdout, our_stderr) =
        progress_parity_impl(&["-r", "--progress"], "progress");

    let (up_line, up_stdout_txt, up_stderr_txt, up_stream) =
        extract_progress_line(&up_stdout, &up_stderr);
    let (our_line, our_stdout_txt, our_stderr_txt, our_stream) =
        extract_progress_line(&our_stdout, &our_stderr);

    assert_progress_stream(up_stream, our_stream);
    assert_non_progress_output(
        &up_stdout_txt,
        &up_stderr_txt,
        &our_stdout_txt,
        &our_stderr_txt,
    );

    let normalized = normalize_progress_line(&our_line);
    assert_eq!(
        normalize_progress_line(&up_line),
        normalized,
        "progress line mismatch",
    );
    insta::assert_snapshot!("progress_parity", normalized);
}

#[test]
fn progress_parity_p() {
    let (up_stdout, up_stderr, our_stdout, our_stderr) =
        progress_parity_impl(&["-r", "-P"], "progress_p");

    let (up_line, up_stdout_txt, up_stderr_txt, up_stream) =
        extract_progress_line(&up_stdout, &up_stderr);
    let (our_line, our_stdout_txt, our_stderr_txt, our_stream) =
        extract_progress_line(&our_stdout, &our_stderr);

    assert_progress_stream(up_stream, our_stream);
    assert_non_progress_output(
        &up_stdout_txt,
        &up_stderr_txt,
        &our_stdout_txt,
        &our_stderr_txt,
    );

    let normalized = normalize_progress_line(&our_line);
    assert_eq!(
        normalize_progress_line(&up_line),
        normalized,
        "progress line mismatch",
    );
    insta::assert_snapshot!("progress_parity_p", normalized);
}

#[test]
fn progress_flag_human_readable() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();

    std::fs::write(src_dir.join("a.txt"), vec![0u8; 2 * 1024]).unwrap();
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    let assert = cmd
        .args([
            "--recursive",
            "--progress",
            "--human-readable",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let text = if !stdout.is_empty() { stdout } else { stderr };
    let mut lines = text.lines();
    assert_eq!(lines.next().unwrap(), "sending incremental file list");
    assert_eq!(lines.next().unwrap(), "a.txt");
    let progress_line = lines.next().unwrap().trim_start_matches('\r').trim_end();
    let bytes = progress_formatter(2 * 1024, true);
    let expected_prefix = format!("{:>15} {:>3}%", bytes, 100);
    assert!(progress_line.starts_with(&expected_prefix));
}
