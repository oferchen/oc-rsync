// crates/daemon/src/lib.rs
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicUsize, atomic::Ordering, Arc};
use std::time::Duration;

#[cfg(unix)]
use nix::errno::Errno;
#[cfg(unix)]
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
#[cfg(unix)]
use nix::unistd::{fork, setsid, ForkResult};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use ipnet::IpNet;
use logging::{DebugFlag, InfoFlag, LogFormat, StderrMode, SubscriberConfig};
use protocol::{negotiate_version, SUPPORTED_PROTOCOLS};
#[cfg(unix)]
use sd_notify::{self, NotifyState};
use transport::{AddressFamily, RateLimitedTransport, TcpTransport, TimeoutTransport, Transport};

fn parse_list(val: &str) -> Vec<String> {
    val.split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn parse_bool(val: &str) -> io::Result<bool> {
    if ["1", "yes", "true", "on"]
        .iter()
        .any(|v| val.eq_ignore_ascii_case(v))
    {
        Ok(true)
    } else if ["0", "no", "false", "off"]
        .iter()
        .any(|v| val.eq_ignore_ascii_case(v))
    {
        Ok(false)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid boolean: {val}"),
        ))
    }
}

#[cfg(unix)]
fn parse_uid(val: &str) -> io::Result<u32> {
    if let Ok(n) = val.parse::<u32>() {
        return Ok(n);
    }
    use nix::unistd::User;
    User::from_name(val)
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unknown user"))
        .map(|u| u.uid.as_raw())
}

#[cfg(not(unix))]
fn parse_uid(val: &str) -> io::Result<u32> {
    val.parse::<u32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(unix)]
fn parse_gid(val: &str) -> io::Result<u32> {
    if let Ok(n) = val.parse::<u32>() {
        return Ok(n);
    }
    use nix::unistd::Group;
    Group::from_name(val)
        .map_err(io::Error::other)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unknown group"))
        .map(|g| g.gid.as_raw())
}

#[cfg(not(unix))]
fn parse_gid(val: &str) -> io::Result<u32> {
    val.parse::<u32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn init_logging(
    log_file: Option<&Path>,
    log_format: Option<&str>,
    syslog: bool,
    journald: bool,
    quiet: bool,
) {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(0)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(quiet)
        .stderr(StderrMode::Errors)
        .log_file(log_file.map(|p| (p.to_path_buf(), log_format.map(|s| s.to_string()))))
        .syslog(syslog)
        .journald(journald)
        .colored(false)
        .timestamps(true)
        .build();
    logging::init(cfg);
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub comment: Option<String>,
    pub hosts_allow: Vec<String>,
    pub hosts_deny: Vec<String>,
    pub auth_users: Vec<String>,
    pub secrets_file: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub use_chroot: bool,
    pub numeric_ids: bool,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub read_only: bool,
    pub write_only: bool,
    pub list: bool,
    pub max_connections: Option<u32>,
    pub refuse_options: Vec<String>,
    pub connections: Arc<AtomicUsize>,
}

impl Clone for Module {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            path: self.path.clone(),
            comment: self.comment.clone(),
            hosts_allow: self.hosts_allow.clone(),
            hosts_deny: self.hosts_deny.clone(),
            auth_users: self.auth_users.clone(),
            secrets_file: self.secrets_file.clone(),
            timeout: self.timeout,
            use_chroot: self.use_chroot,
            numeric_ids: self.numeric_ids,
            uid: self.uid,
            gid: self.gid,
            read_only: self.read_only,
            write_only: self.write_only,
            list: self.list,
            max_connections: self.max_connections,
            refuse_options: self.refuse_options.clone(),
            connections: Arc::clone(&self.connections),
        }
    }
}

impl Default for Module {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::new(),
            comment: None,
            hosts_allow: Vec::new(),
            hosts_deny: Vec::new(),
            auth_users: Vec::new(),
            secrets_file: None,
            timeout: None,
            use_chroot: true,
            numeric_ids: false,
            uid: None,
            gid: None,
            read_only: true,
            write_only: false,
            list: true,
            max_connections: None,
            refuse_options: Vec::new(),
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub fn parse_module(s: &str) -> std::result::Result<Module, String> {
    let mut parts = s.splitn(2, '=');
    let name = parts
        .next()
        .ok_or_else(|| "missing module name".to_string())?
        .trim();
    if name.is_empty() {
        return Err("missing module name".to_string());
    }
    let rest = parts
        .next()
        .ok_or_else(|| "missing module path".to_string())?
        .trim();
    if rest.is_empty() {
        return Err("missing module path".to_string());
    }

    let mut iter = rest.split(',');
    let path_str = iter
        .next()
        .ok_or_else(|| "module path missing or malformed".to_string())?
        .trim();
    if path_str.is_empty() {
        return Err("module path missing or malformed".to_string());
    }
    let raw = PathBuf::from(path_str);
    let abs = if raw.is_absolute() {
        raw
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(raw)
    };

    let mut module = Module {
        name: name.to_string(),
        path: abs,
        ..Module::default()
    };

    for opt in iter {
        let mut kv = opt.splitn(2, '=');
        let key = kv
            .next()
            .ok_or_else(|| "missing option key".to_string())?
            .trim()
            .to_lowercase();
        let val = kv
            .next()
            .ok_or_else(|| format!("missing value for {key}"))?
            .trim();
        let key = key.replace('-', "_");
        match key.as_str() {
            "hosts_allow" => module.hosts_allow = parse_list(val),
            "hosts_deny" => module.hosts_deny = parse_list(val),
            "auth_users" => module.auth_users = parse_list(val),
            "comment" => module.comment = Some(val.to_string()),
            "secrets_file" => module.secrets_file = Some(PathBuf::from(val)),
            "timeout" => {
                let secs = val
                    .parse::<u64>()
                    .map_err(|e| format!("invalid timeout: {e}"))?;
                module.timeout = if secs == 0 {
                    None
                } else {
                    Some(Duration::from_secs(secs))
                };
            }
            "use_chroot" => module.use_chroot = parse_bool(val).map_err(|e| e.to_string())?,
            "numeric_ids" => module.numeric_ids = parse_bool(val).map_err(|e| e.to_string())?,
            "uid" => module.uid = Some(parse_uid(val).map_err(|e| e.to_string())?),
            "gid" => module.gid = Some(parse_gid(val).map_err(|e| e.to_string())?),
            "read_only" => module.read_only = parse_bool(val).map_err(|e| e.to_string())?,
            "write_only" => module.write_only = parse_bool(val).map_err(|e| e.to_string())?,
            "list" => module.list = parse_bool(val).map_err(|e| e.to_string())?,
            "max_connections" => {
                let max = val
                    .parse::<u32>()
                    .map_err(|e| format!("invalid max connections: {e}"))?;
                module.max_connections = Some(max);
            }
            "refuse_options" => module.refuse_options = parse_list(val),
            _ => return Err(format!("unknown option: {key}")),
        }
    }

    Ok(module)
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
    pub pid_file: Option<PathBuf>,
    pub lock_file: Option<PathBuf>,
    pub secrets_file: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub use_chroot: Option<bool>,
    pub numeric_ids: Option<bool>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub read_only: Option<bool>,
    pub write_only: Option<bool>,
    pub list: Option<bool>,
    pub max_connections: Option<usize>,
    pub refuse_options: Vec<String>,
    pub modules: Vec<Module>,
}

pub fn parse_config(contents: &str) -> io::Result<DaemonConfig> {
    let mut cfg = DaemonConfig::default();
    let mut current: Option<Module> = None;
    for raw in contents.lines() {
        let mut line = String::new();
        let mut in_quotes: Option<char> = None;
        let mut prev_ws = true;
        for c in raw.chars() {
            match c {
                '"' | '\'' => {
                    if let Some(q) = in_quotes {
                        if c == q {
                            in_quotes = None;
                        }
                    } else {
                        in_quotes = Some(c);
                    }
                    line.push(c);
                    prev_ws = c.is_whitespace();
                }
                '#' | ';' if in_quotes.is_none() && prev_ws => {
                    break;
                }
                c => {
                    prev_ws = c.is_whitespace();
                    line.push(c);
                }
            }
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(m) = current.take() {
                if m.path.as_os_str().is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("module {} missing path", m.name),
                    ));
                }
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
            .to_lowercase()
            .replace(['-', '_'], " ");
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
            (false, "pid file") => cfg.pid_file = Some(PathBuf::from(val)),
            (false, "lock file") => cfg.lock_file = Some(PathBuf::from(val)),
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
            (false, "use chroot") => {
                cfg.use_chroot = Some(parse_bool(&val)?);
            }
            (false, "numeric ids") => {
                cfg.numeric_ids = Some(parse_bool(&val)?);
            }
            (false, "uid") => {
                cfg.uid = Some(parse_uid(&val)?);
            }
            (false, "gid") => {
                cfg.gid = Some(parse_gid(&val)?);
            }
            (false, "read only") => {
                cfg.read_only = Some(parse_bool(&val)?);
            }
            (false, "write only") => {
                cfg.write_only = Some(parse_bool(&val)?);
            }
            (false, "list") => {
                cfg.list = Some(parse_bool(&val)?);
            }
            (false, "max connections") => {
                let max = val
                    .parse::<usize>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                cfg.max_connections = Some(max);
            }
            (false, "refuse options") => {
                cfg.refuse_options = parse_list(&val);
            }
            (true, "path") => {
                if let Some(m) = current.as_mut() {
                    let raw = PathBuf::from(&val);
                    let abs = if raw.is_absolute() {
                        raw
                    } else {
                        env::current_dir()?.join(raw)
                    };
                    let canon = fs::canonicalize(abs)?;
                    m.path = canon;
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
            (true, "comment") => {
                if let Some(m) = current.as_mut() {
                    m.comment = Some(val.trim().to_string());
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
            (true, "use chroot") => {
                if let Some(m) = current.as_mut() {
                    m.use_chroot = parse_bool(&val)?;
                }
            }
            (true, "numeric ids") => {
                if let Some(m) = current.as_mut() {
                    m.numeric_ids = parse_bool(&val)?;
                }
            }
            (true, "uid") => {
                if let Some(m) = current.as_mut() {
                    m.uid = Some(parse_uid(&val)?);
                }
            }
            (true, "gid") => {
                if let Some(m) = current.as_mut() {
                    m.gid = Some(parse_gid(&val)?);
                }
            }
            (true, "read only") => {
                if let Some(m) = current.as_mut() {
                    m.read_only = parse_bool(&val)?;
                }
            }
            (true, "write only") => {
                if let Some(m) = current.as_mut() {
                    m.write_only = parse_bool(&val)?;
                }
            }
            (true, "list") => {
                if let Some(m) = current.as_mut() {
                    m.list = parse_bool(&val)?;
                }
            }
            (true, "max connections") => {
                if let Some(m) = current.as_mut() {
                    let max = val
                        .parse::<u32>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                    m.max_connections = Some(max);
                }
            }
            (true, "refuse options") => {
                if let Some(m) = current.as_mut() {
                    m.refuse_options = parse_list(&val);
                }
            }
            _ => {}
        }
    }
    if let Some(m) = current.take() {
        if m.path.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("module {} missing path", m.name),
            ));
        }
        cfg.modules.push(m);
    }
    Ok(cfg)
}

pub fn parse_config_file(path: &Path) -> io::Result<DaemonConfig> {
    let contents = fs::read_to_string(path)?;
    parse_config(&contents)
}

pub fn load_config(path: Option<&Path>) -> io::Result<DaemonConfig> {
    let default;
    let path = match path {
        Some(p) => p,
        None => {
            default = env::var("OC_RSYNC_CONFIG_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/etc/oc-rsyncd.conf"));
            &default
        }
    };
    parse_config_file(path)
}

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

pub fn authenticate<T: Transport>(
    t: &mut T,
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
) -> io::Result<Option<File>> {
    let log = if let Some(path) = log_file {
        let fmt = log_format.unwrap_or("%h %m");
        let line = fmt.replace("%h", peer).replace("%m", &module.name);
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(f, "{}", line)?;
        Some(f)
    } else {
        None
    };
    #[cfg(unix)]
    {
        chroot_and_drop_privileges(&module.path, uid, gid, module.use_chroot)?;
    }
    Ok(log)
}

#[cfg(unix)]
pub fn chroot_and_drop_privileges(
    path: &Path,
    uid: u32,
    gid: u32,
    use_chroot: bool,
) -> io::Result<()> {
    use nix::unistd::{chdir, chroot, getegid, geteuid};
    let canon = fs::canonicalize(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("path does not exist: {}", path.display()),
            )
        } else {
            io::Error::new(
                e.kind(),
                format!("failed to canonicalize {}: {e}", path.display()),
            )
        }
    })?;

    let meta = fs::metadata(&canon).map_err(|e| {
        io::Error::new(e.kind(), format!("failed to stat {}: {e}", canon.display()))
    })?;
    if !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("path is not a directory: {}", canon.display()),
        ));
    }

    let euid = geteuid().as_raw();
    let egid = getegid().as_raw();
    if use_chroot && euid != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "chroot requires root",
        ));
    }
    if (uid != euid || gid != egid) && euid != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "dropping privileges requires root",
        ));
    }
    if use_chroot {
        chroot(&canon)
            .map_err(|e| io::Error::other(format!("chroot to {} failed: {e}", canon.display())))?;
        chdir("/").map_err(|e| io::Error::other(format!("chdir failed: {e}")))?;
    } else {
        chdir(&canon)
            .map_err(|e| io::Error::other(format!("chdir to {} failed: {e}", canon.display())))?;
    }
    drop_privileges(uid, gid)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn chroot_and_drop_privileges(
    _path: &Path,
    _uid: u32,
    _gid: u32,
    _use_chroot: bool,
) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
pub fn drop_privileges(uid: u32, gid: u32) -> io::Result<()> {
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "redox",
        target_os = "haiku",
    )))]
    use nix::unistd::setgroups;
    use nix::unistd::{getegid, geteuid, setgid, setuid, Gid, Uid};
    let cur_uid = geteuid().as_raw();
    let cur_gid = getegid().as_raw();
    if (uid != cur_uid || gid != cur_gid) && cur_uid != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "dropping privileges requires root",
        ));
    }
    if gid != cur_gid {
        #[cfg(not(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "redox",
            target_os = "haiku",
        )))]
        {
            setgroups(&[Gid::from_raw(gid)]).map_err(io::Error::other)?;
        }
        setgid(Gid::from_raw(gid)).map_err(io::Error::other)?;
    }
    if uid != cur_uid {
        setuid(Uid::from_raw(uid)).map_err(io::Error::other)?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn drop_privileges(_uid: u32, _gid: u32) -> io::Result<()> {
    Ok(())
}

pub type Handler = dyn Fn(&mut dyn Transport) -> io::Result<()> + Send + Sync;

fn host_matches(ip: &IpAddr, pat: &str) -> bool {
    if pat == "*" {
        return true;
    }
    if let Ok(net) = pat.parse::<IpNet>() {
        return net.contains(ip);
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
    password: Option<&str>,
    log_file: Option<&Path>,
    log_format: Option<&str>,
    motd: Option<&Path>,
    list: bool,
    refuse: &[String],
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
    let latest = SUPPORTED_PROTOCOLS[0];
    transport.send(&latest.to_be_bytes())?;
    negotiate_version(latest, peer_ver).map_err(|e| io::Error::other(e.to_string()))?;
    let (token, global_allowed, no_motd) = authenticate(transport, secrets, password)?;
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
    transport.send(b"@RSYNCD: OK\n")?;
    let mut name_buf = [0u8; 256];
    let n = transport.receive(&mut name_buf)?;
    let name = String::from_utf8_lossy(&name_buf[..n]).trim().to_string();
    if name.is_empty() {
        if list {
            for m in modules.values().filter(|m| m.list) {
                let line = format!("{}\n", m.name);
                transport.send(line.as_bytes())?;
            }
        }
        transport.send(b"\n")?;
        return Ok(());
    }
    if let Some(module) = modules.get(&name) {
        if let Ok(ip) = peer.parse::<IpAddr>() {
            if !host_allowed(&ip, &module.hosts_allow, &module.hosts_deny) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "host denied",
                ));
            }
        }
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
        if !module.auth_users.is_empty() {
            match token.as_deref() {
                Some(tok) if module.auth_users.iter().any(|u| u == tok) => {}
                _ => {
                    let _ = transport.send(b"@ERROR: access denied");
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "unauthorized user",
                    ));
                }
            }
        }
        if let Some(max) = module.max_connections {
            if module.connections.load(Ordering::SeqCst) >= max as usize {
                let _ = transport.send(b"@ERROR: max connections reached");
                return Err(io::Error::other("max connections reached"));
            }
            module.connections.fetch_add(1, Ordering::SeqCst);
        }
        transport.send(b"@RSYNCD: OK\n")?;
        let mut opt_buf = [0u8; 256];
        let mut is_sender = false;
        let mut saw_server = false;
        loop {
            let n = transport.receive(&mut opt_buf)?;
            let opt = String::from_utf8_lossy(&opt_buf[..n]).trim().to_string();
            if opt.is_empty() {
                break;
            }
            if opt == "--sender" {
                is_sender = true;
            }
            if opt == "--server" {
                saw_server = true;
            }
            if let Some(v) = opt.strip_prefix("--log-file=") {
                log_file = Some(PathBuf::from(v));
            } else if let Some(v) = opt.strip_prefix("--log-file-format=") {
                log_format = Some(v.to_string());
            }
            if (opt == "--numeric-ids" && !module.numeric_ids)
                || (opt == "--no-numeric-ids" && module.numeric_ids)
                || refuse.iter().any(|r| opt.contains(r))
                || module.refuse_options.iter().any(|r| opt.contains(r))
            {
                let _ = transport.send(b"@ERROR: option refused");
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "option refused",
                ));
            }
        }
        if module.read_only && saw_server && !is_sender {
            let _ = transport.send(b"@ERROR: read only");
            if module.max_connections.is_some() {
                module.connections.fetch_sub(1, Ordering::SeqCst);
            }
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "read only"));
        }
        if module.write_only && saw_server && is_sender {
            let _ = transport.send(b"@ERROR: write only");
            if module.max_connections.is_some() {
                module.connections.fetch_sub(1, Ordering::SeqCst);
            }
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "write only",
            ));
        }
        if let Some(dur) = module.timeout {
            transport.set_read_timeout(Some(dur))?;
            transport.set_write_timeout(Some(dur))?;
        }
        let m_uid = module.uid.unwrap_or(uid);
        let m_gid = module.gid.unwrap_or(gid);
        let mut log = serve_module(
            transport,
            module,
            peer,
            log_file.as_deref(),
            log_format.as_deref(),
            m_uid,
            m_gid,
        )?;
        if module.max_connections.is_some() {
            module.connections.fetch_sub(1, Ordering::SeqCst);
        }
        let res = handler(transport);
        let flush_res = if let Some(f) = log.as_mut() {
            f.flush()
        } else {
            Ok(())
        };
        let close_res = transport.close();
        res.and(flush_res).and(close_res)
    } else {
        let _ = transport.send(b"@ERROR: unknown module");
        Err(io::Error::new(io::ErrorKind::NotFound, "unknown module"))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_daemon(
    modules: HashMap<String, Module>,
    secrets: Option<PathBuf>,
    password: Option<String>,
    hosts_allow: Vec<String>,
    hosts_deny: Vec<String>,
    log_file: Option<PathBuf>,
    log_format: Option<String>,
    syslog: bool,
    journald: bool,
    motd: Option<PathBuf>,
    pid_file: Option<PathBuf>,
    lock_file: Option<PathBuf>,
    state_dir: Option<PathBuf>,
    timeout: Option<Duration>,
    bwlimit: Option<u64>,
    max_connections: Option<usize>,
    refuse_options: Vec<String>,
    list: bool,
    port: u16,
    address: Option<IpAddr>,
    family: Option<AddressFamily>,
    uid: u32,
    gid: u32,
    handler: Arc<Handler>,
    quiet: bool,
    no_detach: bool,
) -> io::Result<()> {
    #[cfg(not(unix))]
    let _ = no_detach;
    #[cfg(unix)]
    if !no_detach {
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => return Ok(()),
            Ok(ForkResult::Child) => {
                setsid().map_err(io::Error::other)?;
            }
            Err(e) => {
                return Err(io::Error::other(e));
            }
        }
    }
    if let Some(path) = &pid_file {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;
        let _ = writeln!(f, "{}", std::process::id());
    }

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

    init_logging(
        log_file.as_deref(),
        log_format.as_deref(),
        syslog,
        journald,
        quiet,
    );

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

    let (listener, real_port) = match TcpTransport::listen(address, port, family) {
        Ok(v) => v,
        Err(e) => {
            let errno = e.raw_os_error().unwrap_or(1) as u32;
            let status = format!("listen failed: {e}");
            #[cfg(unix)]
            let _ = sd_notify::notify(
                false,
                &[NotifyState::Status(&status), NotifyState::Errno(errno)],
            );
            return Err(e);
        }
    };
    if port == 0 && !quiet {
        println!("{}", real_port);
        io::stdout().flush()?;
    }
    #[cfg(unix)]
    std::thread::spawn(|| {
        let _ = sd_notify::notify(false, &[NotifyState::Ready]);
    });
    let active = Arc::new(AtomicUsize::new(0));
    loop {
        #[cfg(unix)]
        loop {
            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => break,
                Ok(_) => {
                    if max_connections.is_some() {
                        active.fetch_sub(1, Ordering::SeqCst);
                    }
                }
                Err(_) => break,
            }
        }
        let (stream, addr) = TcpTransport::accept(&listener, &hosts_allow, &hosts_deny)?;
        if let Some(max) = max_connections {
            if active.load(Ordering::SeqCst) >= max {
                continue;
            }
            active.fetch_add(1, Ordering::SeqCst);
        }
        let peer = addr.ip().to_string();
        let modules = modules.clone();
        let secrets = secrets.clone();
        let password = password.clone();
        let log_file = log_file.clone();
        let log_format = log_format.clone();
        let motd = motd.clone();
        let handler = handler.clone();
        let refuse = refuse_options.clone();
        let active_conn = active.clone();
        #[cfg(unix)]
        match unsafe { fork() } {
            Ok(ForkResult::Parent { .. }) => {
                drop(stream);
            }
            Ok(ForkResult::Child) => {
                let transport = TcpTransport::from_stream(stream);
                let res = (|| -> io::Result<()> {
                    let t = TimeoutTransport::new(transport, timeout)?;
                    if let Some(limit) = bwlimit {
                        let mut t = RateLimitedTransport::new(t, limit);
                        handle_connection(
                            &mut t,
                            &modules,
                            secrets.as_deref(),
                            password.as_deref(),
                            log_file.as_deref(),
                            log_format.as_deref(),
                            motd.as_deref(),
                            list,
                            &refuse,
                            &peer,
                            uid,
                            gid,
                            &handler,
                        )
                    } else {
                        let mut t = t;
                        handle_connection(
                            &mut t,
                            &modules,
                            secrets.as_deref(),
                            password.as_deref(),
                            log_file.as_deref(),
                            log_format.as_deref(),
                            motd.as_deref(),
                            list,
                            &refuse,
                            &peer,
                            uid,
                            gid,
                            &handler,
                        )
                    }
                })();
                if let Err(e) = res {
                    if !quiet {
                        eprintln!("connection error: {}", e);
                    }
                }
                std::process::exit(0);
            }
            Err(e) => {
                if max_connections.is_some() {
                    active_conn.fetch_sub(1, Ordering::SeqCst);
                }
                if !quiet {
                    eprintln!("fork failed: {}", e);
                }
            }
        }
        #[cfg(not(unix))]
        std::thread::spawn(move || {
            let transport = TcpTransport::from_stream(stream);
            let res = (|| -> io::Result<()> {
                let t = TimeoutTransport::new(transport, timeout)?;
                if let Some(limit) = bwlimit {
                    let mut t = RateLimitedTransport::new(t, limit);
                    handle_connection(
                        &mut t,
                        &modules,
                        secrets.as_deref(),
                        password.as_deref(),
                        log_file.as_deref(),
                        log_format.as_deref(),
                        motd.as_deref(),
                        list,
                        &refuse,
                        &peer,
                        uid,
                        gid,
                        &handler,
                    )
                } else {
                    let mut t = t;
                    handle_connection(
                        &mut t,
                        &modules,
                        secrets.as_deref(),
                        password.as_deref(),
                        log_file.as_deref(),
                        log_format.as_deref(),
                        motd.as_deref(),
                        list,
                        &refuse,
                        &peer,
                        uid,
                        gid,
                        &handler,
                    )
                }
            })();
            if max_connections.is_some() {
                active_conn.fetch_sub(1, Ordering::SeqCst);
            }
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
    use std::path::PathBuf;
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

    use super::{parse_bool, parse_config};

    #[test]
    fn parse_config_invalid_port() {
        let cfg = "port=not-a-number";
        let res = parse_config(cfg);
        assert!(res.is_err());
    }

    #[test]
    fn parse_bool_is_case_insensitive() {
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("Yes").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("No").unwrap());
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

    #[test]
    fn parse_config_module_use_chroot() {
        let cfg = parse_config("[data]\npath=/tmp\nuse chroot=no").unwrap();
        assert!(!cfg.modules[0].use_chroot);
    }

    #[test]
    fn parse_config_module_numeric_ids() {
        let cfg = parse_config("[data]\npath=/tmp\nnumeric ids = yes").unwrap();
        assert!(cfg.modules[0].numeric_ids);
    }

    #[test]
    fn parse_config_module_uid_gid() {
        let cfg = parse_config("[data]\npath=/tmp\nuid=0\ngid=0\n").unwrap();
        assert_eq!(cfg.modules[0].uid, Some(0));
        assert_eq!(cfg.modules[0].gid, Some(0));
    }

    #[test]
    fn parse_config_global_uid_gid() {
        let cfg = parse_config("uid=0\ngid=0\n[data]\npath=/tmp\n").unwrap();
        assert_eq!(cfg.uid, Some(0));
        assert_eq!(cfg.gid, Some(0));
    }

    #[test]
    fn parse_config_pid_and_lock_file() {
        let cfg = parse_config("pid file=/tmp/pid\nlock file=/tmp/lock").unwrap();
        assert_eq!(cfg.pid_file, Some(PathBuf::from("/tmp/pid")));
        assert_eq!(cfg.lock_file, Some(PathBuf::from("/tmp/lock")));
    }

    #[test]
    fn parse_config_global_use_chroot() {
        let cfg = parse_config("use chroot=no\n[data]\npath=/tmp").unwrap();
        assert_eq!(cfg.use_chroot, Some(false));
    }

    #[test]
    fn parse_config_global_numeric_ids() {
        let cfg = parse_config("numeric ids=yes\n[data]\npath=/tmp").unwrap();
        assert_eq!(cfg.numeric_ids, Some(true));
    }

    #[cfg(unix)]
    #[test]
    fn parse_module_accepts_symlinked_dir() {
        let dir = tempdir().unwrap();
        let link = dir.path().join("symlinked");
        symlink(dir.path(), &link).unwrap();
        let module = parse_module(&format!("data={}", link.display())).unwrap();
        assert_eq!(module.path, link);
    }
}
