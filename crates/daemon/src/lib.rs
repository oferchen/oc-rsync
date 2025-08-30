use std::fs;
use std::io::{self};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use transport::Transport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
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
    })
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
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "unauthorized"))
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
