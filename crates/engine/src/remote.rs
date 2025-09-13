// crates/engine/src/remote.rs

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::EngineError;
#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSpec {
    pub path: PathBuf,
    pub trailing_slash: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSpec {
    Local(PathSpec),
    Remote {
        host: String,
        path: PathSpec,
        module: Option<String>,
    },
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).to_string())
}

fn bytes_to_string(bytes: &[u8], what: &str) -> Result<String, EngineError> {
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|_| EngineError::Other(format!("{what} not valid UTF-8")))
}

pub fn parse_remote_spec(input: &OsStr) -> Result<RemoteSpec, EngineError> {
    let bytes = input.as_encoded_bytes();
    let (trailing_slash, s) = if bytes != b"/" && bytes.ends_with(b"/") {
        (true, &bytes[..bytes.len() - 1])
    } else {
        (false, bytes)
    };
    if let Some(rest) = s.strip_prefix(b"rsync://") {
        let mut parts = rest.splitn(2, |&b| b == b'/');
        let host = parts.next().unwrap_or(&[]);
        let mod_path = parts.next().unwrap_or(&[]);
        let mut mp = mod_path.splitn(2, |&b| b == b'/');
        let module = mp.next().unwrap_or(&[]);
        let path = mp.next().unwrap_or(&[]);
        let path = if path.is_empty() { b"." } else { path };
        if host.is_empty() {
            return Err(EngineError::Other("remote host missing".into()));
        }
        if module.is_empty() {
            return Err(EngineError::Other("remote module missing".into()));
        }
        return Ok(RemoteSpec::Remote {
            host: bytes_to_string(host, "remote host")?,
            path: PathSpec {
                path: path_from_bytes(path),
                trailing_slash,
            },
            module: Some(bytes_to_string(module, "remote module")?),
        });
    }
    if !s.is_empty() && s[0] == b'[' {
        if let Some(end) = s.iter().position(|&b| b == b']') {
            let host = &s[1..end];
            if s.get(end + 1) == Some(&b':') {
                let path = &s[end + 2..];
                if host.is_empty() {
                    return Err(EngineError::Other("remote host missing".into()));
                }
                if path.is_empty() || path.first() != Some(&b'/') {
                    return Err(EngineError::Other("remote path missing".into()));
                }
                return Ok(RemoteSpec::Remote {
                    host: bytes_to_string(host, "remote host")?,
                    path: PathSpec {
                        path: path_from_bytes(path),
                        trailing_slash,
                    },
                    module: None,
                });
            }
        }
        return Ok(RemoteSpec::Local(PathSpec {
            path: path_from_bytes(s),
            trailing_slash,
        }));
    }
    if let Some(idx) = s.windows(2).position(|w| w == b"::") {
        let host = &s[..idx];
        let mod_path = &s[idx + 2..];
        let mut parts = mod_path.splitn(2, |&b| b == b'/');
        let module = parts.next().unwrap_or(&[]);
        let path = parts.next().unwrap_or(&[]);
        if host.is_empty() {
            return Err(EngineError::Other("remote host missing".into()));
        }
        if module.is_empty() {
            return Err(EngineError::Other("remote module missing".into()));
        }
        let path = if path.is_empty() { b"." } else { path };
        return Ok(RemoteSpec::Remote {
            host: bytes_to_string(host, "remote host")?,
            path: PathSpec {
                path: path_from_bytes(path),
                trailing_slash,
            },
            module: Some(bytes_to_string(module, "remote module")?),
        });
    }
    if let Some(idx) = s.iter().position(|&b| b == b':') {
        if idx == 1 {
            if s[0].is_ascii_alphabetic()
                && (s.len() == 2 || s.get(2) == Some(&b'/') || s.get(2) == Some(&b'\\'))
            {
                return Ok(RemoteSpec::Local(PathSpec {
                    path: path_from_bytes(s),
                    trailing_slash,
                }));
            }
        }
        let host = &s[..idx];
        let path = &s[idx + 1..];
        if host.is_empty() {
            return Err(EngineError::Other("remote host missing".into()));
        }
        if path.is_empty() {
            return Err(EngineError::Other("remote path missing".into()));
        }
        return Ok(RemoteSpec::Remote {
            host: bytes_to_string(host, "remote host")?,
            path: PathSpec {
                path: path_from_bytes(path),
                trailing_slash,
            },
            module: None,
        });
    }
    Ok(RemoteSpec::Local(PathSpec {
        path: path_from_bytes(s),
        trailing_slash,
    }))
}

pub fn is_remote_spec(path: &OsStr) -> bool {
    matches!(parse_remote_spec(path), Ok(RemoteSpec::Remote { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn classify_remote_specs() {
        let cases = [
            "rsync://host/mod/path",
            "host::mod/path",
            "host:/abs",
            "[::1]:/abs",
        ];
        for c in cases {
            assert!(is_remote_spec(OsStr::new(c)));
            assert!(matches!(
                parse_remote_spec(OsStr::new(c)),
                Ok(RemoteSpec::Remote { .. })
            ));
        }
    }

    #[test]
    fn classify_local_specs() {
        let cases = ["/abs", "./rel", "C:/tmp", "dir/file"];
        for c in cases {
            assert!(!is_remote_spec(OsStr::new(c)));
            assert!(matches!(
                parse_remote_spec(OsStr::new(c)),
                Ok(RemoteSpec::Local(_))
            ));
        }
    }
}
