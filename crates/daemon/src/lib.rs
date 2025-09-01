//! Utilities for parsing rsync daemon configuration files.

use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use protocol::{negotiate_version, LATEST_VERSION};
use transport::{AddressFamily, RateLimitedTransport, TcpTransport, Transport};

fn parse_list(val: &str) -> Vec<String> {
    val.split([' ', ','])
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
    pub timeout: Option<Duration>,
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
    let raw = PathBuf::from(path);
    let abs = if raw.is_absolute() {
        raw
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(raw)
    };
    let canonical = fs::canonicalize(abs).map_err(|e| e.to_string())?;
    Ok(Module {
        name,
        path: canonical,
        hosts_allow: Vec::new(),
        hosts_deny: Vec::new(),
        auth_users: Vec::new(),
        secrets_file: None,
        timeout: None,
    })
}

#[derive(Debug, Default, Clone)]
pub struct DaemonArgs {
    pub address: Option<IpAddr>,
    pub port: u16,
    pub family: Option<AddressFamily>,
}

pub fn parse_daemon_args<I>(args: I) -> io::Result<DaemonArgs>
where
    I: IntoIterator<Item = String>,
{
    let mut opts = DaemonArgs {
        port: 873,
        ..DaemonArgs::default()
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--address" => {
                let val = iter.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "missing value for --address")
                })?;
                opts.address = Some(
                    val.parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                );
            }
            a if a.starts_with("--address=") => {
                let val = &a[10..];
                opts.address = Some(
                    val.parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                );
            }
            "--port" => {
                let val = iter.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "missing value for --port")
                })?;
                opts.port = val
                    .parse()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            }
            a if a.starts_with("--port=") => {
                let val = &a[7..];
                opts.port = val
                    .parse()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            }
            "--ipv4" | "-4" => {
                opts.family = Some(AddressFamily::V4);
            }
            "--ipv6" | "-6" => {
                opts.family = Some(AddressFamily::V6);
            }
            _ => {}
        }
    }
    if let (Some(ip), Some(AddressFamily::V4)) = (opts.address, opts.family) {
        if ip.is_ipv6() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "IPv6 address provided with --ipv4",
            ));
        }
    }
    if let (Some(ip), Some(AddressFamily::V6)) = (opts.address, opts.family) {
        if ip.is_ipv4() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "IPv4 address provided with --ipv6",
            ));
        }
    }
    Ok(opts)
}

#[derive(Debug, Default, Clone)]
pub struct DaemonConfig {
    pub address: Option<IpAddr>,
    pub address6: Option<IpAddr>,
    pub port: Option<u16>,
    pub hosts_allow: Vec<String>,
    pub hosts_deny: Vec<String>,
    pub motd_file: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub secrets_file: Option<PathBuf>,
    pub timeout: Option<Duration>,
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
            (false, "address") => {
                let addr = val
                    .parse::<IpAddr>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if addr.is_ipv4() {
                    cfg.address = Some(addr);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "expected IPv4 address",
                    ));
                }
            }
            (false, "address6") => {
                let addr = val
                    .parse::<IpAddr>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if addr.is_ipv6() {
                    cfg.address6 = Some(addr);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "expected IPv6 address",
                    ));
                }
            }
            (false, "port") => {
                let port = val
                    .parse::<u16>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                cfg.port = Some(port);
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
            (false, "timeout") => {
                let secs = val
                    .parse::<u64>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                cfg.timeout = if secs == 0 {
                    None
                } else {
                    Some(Duration::from_secs(secs))
                };
            }
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
            (true, "timeout") => {
                if let Some(m) = current.as_mut() {
                    let secs = val
                        .parse::<u64>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    m.timeout = if secs == 0 {
                        None
                    } else {
                        Some(Duration::from_secs(secs))
                    };
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
    for raw in contents.lines() {
        let line = raw.split(['#', ';']).next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
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
        if token_str.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "missing token",
            ));
        }
        let allowed = authenticate_token(&token_str, auth_path)?;
        Ok((Some(token_str), allowed, no_motd))
    } else {
        let token_opt = if token_str.is_empty() {
            None
        } else {
            Some(token_str)
        };
        Ok((token_opt, Vec::new(), no_motd))
    }
}

pub fn serve_module<T: Transport>(
    _t: &mut T,
    module: &Module,
    peer: &str,
    log_file: Option<&Path>,
    log_format: Option<&str>,
    uid: u32,
    gid: u32,
) -> io::Result<()> {
    if let Some(path) = log_file {
        let fmt = log_format.unwrap_or("%h %m");
        let line = fmt.replace("%h", peer).replace("%m", &module.name);
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(f, "{}", line)?;
        f.flush()?;
    }
    #[cfg(unix)]
    {
        chroot_and_drop_privileges(&module.path, uid, gid)?;
    }
    Ok(())
}

#[cfg(unix)]
pub fn chroot_and_drop_privileges(path: &Path, uid: u32, gid: u32) -> io::Result<()> {
    use nix::unistd::{chdir, chroot, setgid, setuid, Gid, Uid};
    chroot(path).map_err(io::Error::other)?;
    chdir("/").map_err(io::Error::other)?;
    setgid(Gid::from_raw(gid)).map_err(io::Error::other)?;
    setuid(Uid::from_raw(uid)).map_err(io::Error::other)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn chroot_and_drop_privileges(_path: &Path, _uid: u32, _gid: u32) -> io::Result<()> {
    Ok(())
}

pub type Handler = dyn Fn(&mut dyn Transport) -> io::Result<()> + Send + Sync;

fn host_matches(ip: &IpAddr, pat: &str) -> bool {
    if pat == "*" {
        return true;
    }
    pat.parse::<IpAddr>().is_ok_and(|p| &p == ip)
}

pub fn host_allowed(ip: &IpAddr, allow: &[String], deny: &[String]) -> bool {
    if !allow.is_empty() && !allow.iter().any(|p| host_matches(ip, p)) {
        return false;
    }
    if deny.iter().any(|p| host_matches(ip, p)) {
        return false;
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub fn handle_connection<T: Transport>(
    transport: &mut T,
    modules: &HashMap<String, Module>,
    secrets: Option<&Path>,
    log_file: Option<&Path>,
    log_format: Option<&str>,
    motd: Option<&Path>,
    peer: &str,
    uid: u32,
    gid: u32,
    handler: &Arc<Handler>,
) -> io::Result<()> {
    let mut log_file = log_file.map(|p| p.to_path_buf());
    let mut log_format = log_format.map(|s| s.to_string());
    let mut buf = [0u8; 4];
    let n = transport.receive(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let peer_ver = u32::from_be_bytes(buf);
    transport.send(&LATEST_VERSION.to_be_bytes())?;
    negotiate_version(LATEST_VERSION, peer_ver).map_err(|e| io::Error::other(e.to_string()))?;

    let (mut token, global_allowed, no_motd) = authenticate(transport, secrets)?;

    if !no_motd {
        if let Some(mpath) = motd {
            if let Ok(content) = fs::read_to_string(mpath) {
                for line in content.lines() {
                    let msg = format!("@RSYNCD: {line}\n");
                    transport.send(msg.as_bytes())?;
                }
            }
        }
    }
    let name = if token.is_some() && global_allowed.is_empty() && secrets.is_none() {
        token.take().unwrap()
    } else {
        let mut name_buf = [0u8; 256];
        let n = transport.receive(&mut name_buf)?;
        String::from_utf8_lossy(&name_buf[..n]).trim().to_string()
    };
    if let Some(module) = modules.get(&name) {
        if let Ok(ip) = peer.parse::<IpAddr>() {
            if !host_allowed(&ip, &module.hosts_allow, &module.hosts_deny) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "host denied",
                ));
            }
        }
    }
    transport.send(b"@RSYNCD: OK\n")?;

    let mut opt_buf = [0u8; 256];
    loop {
        let n = transport.receive(&mut opt_buf)?;
        let opt = String::from_utf8_lossy(&opt_buf[..n]).trim().to_string();
        if opt.is_empty() {
            break;
        }
        if let Some(v) = opt.strip_prefix("--log-file=") {
            log_file = Some(PathBuf::from(v));
        } else if let Some(v) = opt.strip_prefix("--log-file-format=") {
            log_format = Some(v.to_string());
        }
    }
    if let Some(module) = modules.get(&name) {
        let allowed = if let Some(path) = module.secrets_file.as_deref() {
            match token.as_deref() {
                Some(tok) => authenticate_token(tok, path)?,
                None => {
                    let _ = transport.send(b"@ERROR: access denied");
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "missing token",
                    ));
                }
            }
        } else {
            global_allowed.clone()
        };
        if !allowed.is_empty() && !allowed.iter().any(|m| m == &name) {
            let _ = transport.send(b"@ERROR: access denied");
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unauthorized module",
            ));
        }
        serve_module(
            transport,
            module,
            peer,
            log_file.as_deref(),
            log_format.as_deref(),
            uid,
            gid,
        )?;
        handler(transport)
    } else {
        let _ = transport.send(b"@ERROR: unknown module");
        Err(io::Error::new(io::ErrorKind::NotFound, "unknown module"))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_daemon(
    modules: HashMap<String, Module>,
    secrets: Option<PathBuf>,
    hosts_allow: Vec<String>,
    hosts_deny: Vec<String>,
    log_file: Option<PathBuf>,
    log_format: Option<String>,
    motd: Option<PathBuf>,
    lock_file: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    timeout: Option<Duration>,
    bwlimit: Option<u64>,
    port: u16,
    address: Option<IpAddr>,
    family: Option<AddressFamily>,
    uid: u32,
    gid: u32,
    handler: Arc<Handler>,
    quiet: bool,
) -> io::Result<()> {
    let _lock = if let Some(path) = lock_file {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let _ = writeln!(f, "{}", std::process::id());
        Some(f)
    } else {
        None
    };

    if let Some(dir) = state_dir {
        let _ = fs::create_dir_all(dir);
    }

    if let Some(addr) = address {
        if let Some(AddressFamily::V4) = family {
            if addr.is_ipv6() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "IPv6 address provided with --ipv4",
                ));
            }
        }
        if let Some(AddressFamily::V6) = family {
            if addr.is_ipv4() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "IPv4 address provided with --ipv6",
                ));
            }
        }
    }

    let (listener, real_port) = TcpTransport::listen(address, port, family)?;
    if port == 0 && !quiet {
        println!("{}", real_port);
        io::stdout().flush()?;
    }
    loop {
        let (stream, addr) = TcpTransport::accept(&listener, &hosts_allow, &hosts_deny)?;
        let peer = addr.ip().to_string();
        let modules = modules.clone();
        let secrets = secrets.clone();
        let log_file = log_file.clone();
        let log_format = log_format.clone();
        let motd = motd.clone();
        let handler = handler.clone();
        std::thread::spawn(move || {
            let mut transport = TcpTransport::from_stream(stream);
            let _ = transport.set_read_timeout(timeout);
            let res = if let Some(limit) = bwlimit {
                let mut t = RateLimitedTransport::new(transport, limit);
                handle_connection(
                    &mut t,
                    &modules,
                    secrets.as_deref(),
                    log_file.as_deref(),
                    log_format.as_deref(),
                    motd.as_deref(),
                    &peer,
                    uid,
                    gid,
                    &handler,
                )
            } else {
                handle_connection(
                    &mut transport,
                    &modules,
                    secrets.as_deref(),
                    log_file.as_deref(),
                    log_format.as_deref(),
                    motd.as_deref(),
                    &peer,
                    uid,
                    gid,
                    &handler,
                )
            };
            if let Err(e) = res {
                if !quiet {
                    eprintln!("connection error: {}", e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{self, Read};
    use std::time::Duration;
    use tempfile::tempdir;
    use transport::LocalPipeTransport;

    #[cfg(unix)]
    use std::os::unix::fs::{symlink, PermissionsExt};

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

    #[test]
    fn parse_config_allows_zero_port() {
        let cfg = parse_config("port=0").unwrap();
        assert_eq!(cfg.port, Some(0));
    }

    #[test]
    fn parse_config_accepts_ipv4_address() {
        let cfg = parse_config("address=127.0.0.1").unwrap();
        assert_eq!(cfg.address, Some("127.0.0.1".parse().unwrap()));
        assert!(cfg.address6.is_none());
    }

    #[test]
    fn parse_config_accepts_ipv6_address() {
        let cfg = parse_config("address6=::1").unwrap();
        assert_eq!(cfg.address6, Some("::1".parse().unwrap()));
        assert!(cfg.address.is_none());
    }

    #[test]
    fn parse_config_sets_timeout() {
        let cfg = parse_config("timeout=5").unwrap();
        assert_eq!(cfg.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn parse_config_module_timeout() {
        let cfg = parse_config("[data]\npath=/tmp\ntimeout=7").unwrap();
        assert_eq!(cfg.modules[0].timeout, Some(Duration::from_secs(7)));
    }

    #[cfg(unix)]
    #[test]
    fn parse_module_accepts_symlinked_dir() {
        let dir = tempdir().unwrap();
        let link = dir.path().join("symlinked");
        symlink(dir.path(), &link).unwrap();
        let module = parse_module(&format!("data={}", link.display())).unwrap();
        assert_eq!(module.path, fs::canonicalize(&link).unwrap());
    }
}
