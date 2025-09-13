// crates/filters/src/parser/util.rs
#![allow(clippy::while_let_on_iterator)]

use std::fs;
use std::io::{self, Read};
use std::path::Path;

pub fn decode_line(raw: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    let mut escaped = false;
    let mut started = false;
    let mut last_non_space = 0;
    while let Some(c) = chars.next() {
        if escaped {
            out.push(c);
            last_non_space = out.len();
            escaped = false;
            continue;
        }
        if !started {
            if c.is_whitespace() {
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '#' {
                return None;
            }
            started = true;
            out.push(c);
            if !c.is_whitespace() {
                last_non_space = out.len();
            }
        } else if c == '\\' {
            escaped = true;
        } else {
            out.push(c);
            if !c.is_whitespace() {
                last_non_space = out.len();
            }
        }
    }
    if escaped {
        out.push('\\');
        last_non_space = out.len();
    }
    out.truncate(last_non_space);
    if out.is_empty() { None } else { Some(out) }
}

pub fn trim_newlines(mut s: &[u8]) -> &[u8] {
    while let Some(&b) = s.last() {
        if b == b'\n' || b == b'\r' {
            s = &s[..s.len() - 1];
        } else {
            break;
        }
    }
    s
}

pub fn read_path_or_stdin(path: &Path) -> io::Result<Vec<u8>> {
    if path == Path::new("-") {
        let mut buf = Vec::new();
        std::io::stdin().lock().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        fs::read(path)
    }
}

#[cfg(test)]
mod tests {
    use super::read_path_or_stdin;
    use std::io::{Seek, SeekFrom, Write};
    #[cfg(unix)]
    use std::os::unix::io::IntoRawFd;
    use std::path::Path;
    use tempfile::{NamedTempFile, tempfile};

    #[test]
    fn reads_from_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "hello world").unwrap();
        let data = read_path_or_stdin(tmp.path()).unwrap();
        assert_eq!(data, b"hello world");
    }

    #[cfg(unix)]
    #[test]
    fn reads_from_stdin() {
        let mut file = tempfile().unwrap();
        write!(file, "stdin data").unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        // SAFETY: duplicating `STDIN_FILENO` yields a new valid descriptor.
        let stdin_fd = unsafe { libc::dup(0) };
        assert!(stdin_fd >= 0);

        let file_fd = file.into_raw_fd();
        // SAFETY: both `file_fd` and descriptor `0` are valid for `dup2`.
        assert!(unsafe { libc::dup2(file_fd, 0) } >= 0);
        // SAFETY: `file_fd` is no longer needed after duplication.
        unsafe { libc::close(file_fd) };

        let data = read_path_or_stdin(Path::new("-")).unwrap();

        // SAFETY: restore original stdin from `stdin_fd`.
        assert!(unsafe { libc::dup2(stdin_fd, 0) } >= 0);
        // SAFETY: `stdin_fd` was obtained from `dup` and must be closed.
        unsafe { libc::close(stdin_fd) };

        assert_eq!(data, b"stdin data");
    }
}
