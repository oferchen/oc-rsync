// crates/daemon/src/auth.rs
use std::fs;
use std::io;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use transport::Transport;

pub fn parse_auth_token(token: &str, contents: &str) -> Option<Vec<String>> {
    for raw in contents.lines() {
        let mut in_single = false;
        let mut in_double = false;
        let mut end = raw.len();
        for (i, ch) in raw.char_indices() {
            match ch {
                '\'' if !in_double => in_single = !in_single,
                '"' if !in_single => in_double = !in_double,
                '#' | ';' if !in_single && !in_double => {
                    end = i;
                    break;
                }
                _ => {}
            }
        }
        let line = raw[..end].trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line
            .split_whitespace()
            .map(|s| s.trim_matches(&['"', '\''][..]));
        if let Some(tok) = parts.next() {
            if tok == token {
                return Some(parts.map(|s| s.to_string()).collect());
            }
        }
    }
    None
}

pub fn authenticate_token(token: &str, path: &Path) -> io::Result<Vec<String>> {
    #[cfg(unix)]
    {
        let mode = fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "auth file permissions are too open",
            ));
        }
    }
    let contents = fs::read_to_string(path)?;
    if let Some(allowed) = parse_auth_token(token, &contents) {
        Ok(allowed)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unauthorized",
        ))
    }
}

pub fn authenticate(
    t: &mut dyn Transport,
    path: Option<&Path>,
    password: Option<&str>,
) -> io::Result<(Option<String>, Vec<String>, bool)> {
    let mut no_motd = false;
    const MAX_TOKEN: usize = 256;
    let mut token = Vec::new();
    let mut buf = [0u8; 64];
    loop {
        let n = t.receive(&mut buf)?;
        if n == 0 {
            if token.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "missing token",
                ));
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "missing terminator",
                ));
            }
        }
        let mut start = 0;
        if token.is_empty() && buf[0] == 0 {
            no_motd = true;
            start = 1;
            if start >= n {
                continue;
            }
        }
        if let Some(pos) = buf[start..n].iter().position(|&b| b == b'\n') {
            token.extend_from_slice(&buf[start..start + pos]);
            if token.len() > MAX_TOKEN {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "token too long"));
            }
            break;
        } else {
            token.extend_from_slice(&buf[start..n]);
            if token.len() > MAX_TOKEN {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "token too long"));
            }
        }
    }
    let token_str = String::from_utf8_lossy(&token).trim().to_string();

    if let Some(auth_path) = path {
        if !auth_path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "auth file missing"));
        }
        if token_str.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "missing token",
            ));
        }
        let allowed = authenticate_token(&token_str, auth_path)?;
        Ok((Some(token_str), allowed, no_motd))
    } else if let Some(pw) = password {
        if token_str.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "missing token",
            ));
        }
        if token_str != pw {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unauthorized",
            ));
        }
        Ok((Some(token_str), Vec::new(), no_motd))
    } else if token_str.is_empty() {
        Ok((None, Vec::new(), no_motd))
    } else {
        Ok((Some(token_str), Vec::new(), no_motd))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use tempfile::tempdir;
    use transport::LocalPipeTransport;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    struct ChunkReader {
        data: Vec<u8>,
        pos: usize,
        chunk: usize,
    }

    impl io::Read for ChunkReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let end = (self.pos + self.chunk).min(self.data.len());
            let len = end - self.pos;
            buf[..len].copy_from_slice(&self.data[self.pos..end]);
            self.pos = end;
            Ok(len)
        }
    }

    #[test]
    fn authenticate_handles_split_reads() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth");
        fs::write(&auth_path, "secret user\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&auth_path, fs::Permissions::from_mode(0o600)).unwrap();

        let reader = ChunkReader {
            data: b"secret\n".to_vec(),
            pos: 0,
            chunk: 1,
        };
        let writer = io::sink();
        let mut t = LocalPipeTransport::new(reader, writer);
        let (_tok, allowed, no_motd) = authenticate(&mut t, Some(&auth_path), None).unwrap();
        assert!(!no_motd);
        assert_eq!(allowed, vec!["user".to_string()]);
    }

    #[test]
    fn authenticate_rejects_long_token() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth");
        fs::write(&auth_path, "tok user\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&auth_path, fs::Permissions::from_mode(0o600)).unwrap();

        let mut data = vec![b'a'; 257];
        data.push(b'\n');
        let reader = std::io::Cursor::new(data);
        let writer = io::sink();
        let mut t = LocalPipeTransport::new(reader, writer);
        let err = authenticate(&mut t, Some(&auth_path), None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(err.to_string(), "token too long");
    }

    #[test]
    fn authenticate_missing_token() {
        let reader = std::io::Cursor::new(b"\n".to_vec());
        let writer = io::sink();
        let mut t = LocalPipeTransport::new(reader, writer);
        let (tok, allowed, no_motd) = authenticate(&mut t, None, None).unwrap();
        assert!(tok.is_none());
        assert!(allowed.is_empty());
        assert!(!no_motd);
    }

    #[test]
    fn authenticate_wrong_token() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth");
        fs::write(&auth_path, "secret user\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&auth_path, fs::Permissions::from_mode(0o600)).unwrap();

        let reader = std::io::Cursor::new(b"wrong\n".to_vec());
        let writer = io::sink();
        let mut t = LocalPipeTransport::new(reader, writer);
        let err = authenticate(&mut t, Some(&auth_path), None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }
}
