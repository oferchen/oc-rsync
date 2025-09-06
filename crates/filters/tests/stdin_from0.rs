// crates/filters/tests/stdin_from0.rs
use filters::{parse_file, Matcher};
use std::collections::HashSet;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::{tempdir, tempfile};

#[cfg(unix)]
use std::os::unix::io::IntoRawFd;

#[cfg(unix)]
#[test]
fn null_separated_filters_from_stdin_match_rsync() {
    let mut tmpfile = tempfile().unwrap();
    tmpfile.write_all(b"+ foo\0+ bar\0- *\0").unwrap();
    tmpfile.seek(SeekFrom::Start(0)).unwrap();

    let stdin_fd = unsafe { libc::dup(0) };
    let file_fd = tmpfile.into_raw_fd();
    assert!(unsafe { libc::dup2(file_fd, 0) } >= 0);
    unsafe { libc::close(file_fd) };

    let mut visited = HashSet::new();
    let rules = parse_file(Path::new("-"), false, &mut visited, 0).unwrap();
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());

    assert!(unsafe { libc::dup2(stdin_fd, 0) } >= 0);
    unsafe { libc::close(stdin_fd) };

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("foo"), "").unwrap();
    fs::write(src.join("bar"), "").unwrap();
    fs::write(src.join("baz"), "").unwrap();
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();

    let mut child = Command::new("rsync")
        .arg("-r")
        .arg("-n")
        .arg("-i")
        .arg("--from0")
        .arg("--filter=merge,-")
        .arg(format!("{}/", src.display()))
        .arg(dest.to_str().unwrap())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"+ foo\0+ bar\0- *\0").unwrap();
    }
    let output = child.wait_with_output().unwrap();
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
    assert_eq!(
        matcher.is_included("foo").unwrap(),
        rsync_included.contains(&"foo".to_string())
    );
    assert_eq!(
        matcher.is_included("bar").unwrap(),
        rsync_included.contains(&"bar".to_string())
    );
    assert_eq!(
        matcher.is_included("baz").unwrap(),
        rsync_included.contains(&"baz".to_string())
    );
}
