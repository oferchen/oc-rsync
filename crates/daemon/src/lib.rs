use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use transport::Transport;

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
            (false, "port") => cfg.port = val.parse().ok(),
            (false, "motd file") => cfg.motd_file = Some(PathBuf::from(val)),
            (false, "log file") => cfg.log_file = Some(PathBuf::from(val)),
            (false, "hosts allow") => {
                cfg.hosts_allow = val
                    .split(|c| c == ' ' || c == ',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            (false, "hosts deny") => {
                cfg.hosts_deny = val
                    .split(|c| c == ' ' || c == ',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            (false, "secrets file") => cfg.secrets_file = Some(PathBuf::from(val)),
            (true, "path") => {
                if let Some(m) = current.as_mut() {
                    m.path = PathBuf::from(val);
                }
            }
            (true, "hosts allow") => {
                if let Some(m) = current.as_mut() {
                    m.hosts_allow = val
                        .split(|c| c == ' ' || c == ',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                }
            }
            (true, "hosts deny") => {
                if let Some(m) = current.as_mut() {
                    m.hosts_deny = val
                        .split(|c| c == ' ' || c == ',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                }
            }
            (true, "auth users") => {
                if let Some(m) = current.as_mut() {
                    m.auth_users = val
                        .split(|c| c == ' ' || c == ',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
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

pub fn authenticate<T: Transport>(t: &mut T, path: Option<&Path>) -> io::Result<Vec<String>> {
    let auth_path = path.unwrap_or(Path::new("auth"));
    if !auth_path.exists() {
        return Ok(Vec::new());
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
    let mut buf = [0u8; 256];
    let n = t.receive(&mut buf)?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "missing token",
        ));
    }
    let token = String::from_utf8_lossy(&buf[..n]).trim().to_string();
    if let Some(allowed) = parse_auth_token(&token, &contents) {
        Ok(allowed)
    } else {
        let _ = t.send(b"@ERROR: access denied");
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unauthorized",
        ))
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
