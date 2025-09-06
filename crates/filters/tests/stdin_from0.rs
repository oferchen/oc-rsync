// crates/filters/tests/stdin_from0.rs
use filters::{Matcher, parse_file};
use std::collections::HashSet;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::tempfile;

#[cfg(unix)]
use std::os::unix::io::IntoRawFd;

#[cfg(unix)]
#[test]
fn null_separated_filters_from_stdin() {
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

    assert!(unsafe { libc::dup2(stdin_fd, 0) } >= 0);
    unsafe { libc::close(stdin_fd) };

    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}
