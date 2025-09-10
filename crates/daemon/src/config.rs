// crates/daemon/src/config.rs
use std::env;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Duration;

use transport::AddressFamily;

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

pub struct ModuleBuilder {
    inner: Module,
}

impl ModuleBuilder {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            inner: Module {
                name: name.into(),
                path: path.into(),
                ..Module::default()
            },
        }
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.inner.comment = Some(comment.into());
        self
    }

    pub fn hosts_allow(mut self, hosts: Vec<String>) -> Self {
        self.inner.hosts_allow = hosts;
        self
    }

    pub fn hosts_deny(mut self, hosts: Vec<String>) -> Self {
        self.inner.hosts_deny = hosts;
        self
    }

    pub fn auth_users(mut self, users: Vec<String>) -> Self {
        self.inner.auth_users = users;
        self
    }

    pub fn secrets_file(mut self, path: PathBuf) -> Self {
        self.inner.secrets_file = Some(path);
        self
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.inner.timeout = timeout;
        self
    }

    pub fn use_chroot(mut self, use_chroot: bool) -> Self {
        self.inner.use_chroot = use_chroot;
        self
    }

    pub fn numeric_ids(mut self, numeric: bool) -> Self {
        self.inner.numeric_ids = numeric;
        self
    }

    pub fn uid(mut self, uid: u32) -> Self {
        self.inner.uid = Some(uid);
        self
    }

    pub fn gid(mut self, gid: u32) -> Self {
        self.inner.gid = Some(gid);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.inner.read_only = read_only;
        self
    }

    pub fn write_only(mut self, write_only: bool) -> Self {
        self.inner.write_only = write_only;
        self
    }

    pub fn list(mut self, list: bool) -> Self {
        self.inner.list = list;
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.inner.max_connections = Some(max);
        self
    }

    pub fn refuse_options(mut self, refuse: Vec<String>) -> Self {
        self.inner.refuse_options = refuse;
        self
    }

    pub fn build(self) -> Module {
        self.inner
    }
}

impl Module {
    pub fn builder(name: impl Into<String>, path: impl Into<PathBuf>) -> ModuleBuilder {
        ModuleBuilder::new(name, path)
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
            (false, "motd file") => {
                cfg.motd_file = if val.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(val))
                }
            }
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
            (false, "use chroot") => cfg.use_chroot = Some(parse_bool(&val)?),
            (false, "numeric ids") => cfg.numeric_ids = Some(parse_bool(&val)?),
            (false, "uid") => cfg.uid = Some(parse_uid(&val)?),
            (false, "gid") => cfg.gid = Some(parse_gid(&val)?),
            (false, "read only") => cfg.read_only = Some(parse_bool(&val)?),
            (false, "write only") => cfg.write_only = Some(parse_bool(&val)?),
            (false, "list") => cfg.list = Some(parse_bool(&val)?),
            (false, "max connections") => {
                let max = val
                    .parse::<usize>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                cfg.max_connections = Some(max);
            }
            (false, "refuse options") => cfg.refuse_options = parse_list(&val),
            (true, "path") => {
                if let Some(m) = current.as_mut() {
                    let p = PathBuf::from(val);
                    m.path = fs::canonicalize(&p).unwrap_or(p);
                }
            }
            (true, "comment") => {
                if let Some(m) = current.as_mut() {
                    m.comment = Some(val);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

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

    #[test]
    fn module_builder_defaults() {
        let module = Module::builder("data", "/tmp").build();
        assert!(module.read_only);
        assert!(module.list);
        assert!(module.use_chroot);
        assert!(!module.numeric_ids);
    }

    #[test]
    fn parse_config_module_defaults() {
        let cfg = parse_config("[data]\npath=/tmp").unwrap();
        let module = &cfg.modules[0];
        assert!(module.read_only);
        assert!(module.list);
        assert!(module.use_chroot);
    }
}
