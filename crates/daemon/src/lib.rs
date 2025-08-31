//! Utilities for parsing rsync daemon configuration files.
//!
//! `parse_config` returns an [`io::Error`] when configuration entries are
//! invalid, such as when the `port` value cannot be parsed.

use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use transport::Transport;

fn parse_list(val: &str) -> Vec<String> {
    val.split(|c| c == ' ' || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub hosts_allow: Vec<String>,
    pub hosts_deny: Vec<String>,
    pub auth_users: Vec<String>,
    pub secrets_file: Option<PathBuf>,
}

pub fn parse_module(s: &str) -> std::result::Result<Module, String> {
    let mut parts = s.splitn(2, '=');
    let name = parts
        .next()
        .ok_or_else(|| "missing module name".to_string())?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| "missing module path".to_string())?;
    Ok(Module {
        name,
        path: PathBuf::from(path),
        hosts_allow: Vec::new(),
        hosts_deny: Vec::new(),
        auth_users: Vec::new(),
        secrets_file: None,
    })
}

#[derive(Debug, Default, Clone)]
pub struct DaemonConfig {
    pub port: Option<u16>,
    pub hosts_allow: Vec<String>,
    pub hosts_deny: Vec<String>,
    pub motd_file: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub secrets_file: Option<PathBuf>,
    pub modules: Vec<Module>,
}

pub fn parse_config(contents: &str) -> io::Result<DaemonConfig> {
    let mut cfg = DaemonConfig::default();
    let mut current: Option<Module> = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(m) = current.take() {
                cfg.modules.push(m);
            }
            let name = line[1..line.len() - 1].trim().to_string();
            current = Some(Module {
                name,
                path: PathBuf::new(),
                ..Module::default()
            });
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing key"))?
            .trim()
            .to_lowercase();
        let val = parts
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing value"))?
            .trim()
            .to_string();
        match (current.is_some(), key.as_str()) {
            (false, "port") => {
                cfg.port = Some(
                    val.parse::<u16>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                );
            }
            (false, "motd file") => cfg.motd_file = Some(PathBuf::from(val)),
            (false, "log file") => cfg.log_file = Some(PathBuf::from(val)),
            (false, "hosts allow") => {
                cfg.hosts_allow = parse_list(&val);
            }
            (false, "hosts deny") => {
                cfg.hosts_deny = parse_list(&val);
            }
            (false, "secrets file") => cfg.secrets_file = Some(PathBuf::from(val)),
            (true, "path") => {
                if let Some(m) = current.as_mut() {
                    m.path = PathBuf::from(val);
                }
            }
            (true, "hosts allow") => {
                if let Some(m) = current.as_mut() {
                    m.hosts_allow = parse_list(&val);
                }
            }
            (true, "hosts deny") => {
                if let Some(m) = current.as_mut() {
                    m.hosts_deny = parse_list(&val);
                }
            }
            (true, "auth users") => {
                if let Some(m) = current.as_mut() {
                    m.auth_users = parse_list(&val);
                }
            }
            (true, "secrets file") => {
                if let Some(m) = current.as_mut() {
                    m.secrets_file = Some(PathBuf::from(val));
                }
            }
            _ => {}
        }
    }
    if let Some(m) = current.take() {
        cfg.modules.push(m);
    }
    Ok(cfg)
}

pub fn parse_config_file(path: &Path) -> io::Result<DaemonConfig> {
    let contents = fs::read_to_string(path)?;
    parse_config(&contents)
}

pub fn parse_auth_token(token: &str, contents: &str) -> Option<Vec<String>> {
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
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

pub fn authenticate<T: Transport>(
    t: &mut T,
    path: Option<&Path>,
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
        #[cfg(unix)]
        {
            let mode = fs::metadata(auth_path)?.permissions().mode();
            if mode & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "auth file permissions are too open",
                ));
            }
        }
        let contents = fs::read_to_string(auth_path)?;
        if let Some(allowed) = parse_auth_token(&token_str, &contents) {
            Ok((Some(token_str), allowed, no_motd))
        } else {
            let _ = t.send(b"@ERROR: access denied");
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unauthorized",
            ))
        }
    } else {
        let token_opt = if token_str.is_empty() {
            None
        } else {
            Some(token_str)
        };
        Ok((token_opt, Vec::new(), no_motd))
    }
}

#[cfg(unix)]
pub fn chroot_and_drop_privileges(path: &Path, uid: u32, gid: u32) -> io::Result<()> {
    use nix::unistd::{chdir, chroot, setgid, setuid, Gid, Uid};
    chroot(path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    chdir("/").map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    setgid(Gid::from_raw(gid)).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    setuid(Uid::from_raw(uid)).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn chroot_and_drop_privileges(_path: &Path, _uid: u32, _gid: u32) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{self, Read};
    use tempfile::tempdir;
    use transport::LocalPipeTransport;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    struct ChunkReader {
        data: Vec<u8>,
        pos: usize,
        chunk: usize,
    }

    impl Read for ChunkReader {
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
        let (_tok, allowed, no_motd) = authenticate(&mut t, Some(&auth_path)).unwrap();
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
        let err = authenticate(&mut t, Some(&auth_path)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(err.to_string(), "token too long");
    }

    use super::parse_config;

    #[test]
    fn parse_config_invalid_port() {
        let cfg = "port=not-a-number";
        let res = parse_config(cfg);
        assert!(res.is_err());
    }
}
