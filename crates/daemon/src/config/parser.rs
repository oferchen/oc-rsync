// crates/daemon/src/config/parser.rs

use std::env;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::model::{DaemonArgs, DaemonConfig, Module};
use super::validator::{parse_bool, parse_gid, parse_uid, validate_daemon_args, validate_module};
use transport::AddressFamily;

fn parse_list(val: &str) -> Vec<String> {
    val.split([' ', ','])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn parse_module(s: &str) -> std::result::Result<Module, String> {
    let mut chars = s.chars().peekable();
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if c == '=' {
            chars.next();
            break;
        }
        name.push(c);
        chars.next();
    }
    let name = name.trim();
    if name.is_empty() {
        return Err("missing module name".to_string());
    }

    let mut path = String::new();
    let mut in_quotes: Option<char> = None;
    for c in chars.by_ref() {
        match c {
            '\'' | '"' => {
                if let Some(q) = in_quotes {
                    if c == q {
                        in_quotes = None;
                    }
                } else {
                    in_quotes = Some(c);
                }
                path.push(c);
            }
            ',' if in_quotes.is_none() => {
                break;
            }
            _ => path.push(c),
        }
    }
    if in_quotes.is_some() {
        return Err("unterminated quote in module path".to_string());
    }
    let path = path.trim();
    if path.is_empty() {
        return Err("missing module path".to_string());
    }
    let path_str = if (path.starts_with('"') && path.ends_with('"'))
        || (path.starts_with('\'') && path.ends_with('\''))
    {
        &path[1..path.len() - 1]
    } else {
        path
    };
    let raw = PathBuf::from(path_str);
    let abs = if raw.is_absolute() {
        raw
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(raw)
    };

    let rest: String = chars.collect();
    let mut module = Module {
        name: name.to_string(),
        path: abs,
        ..Module::default()
    };

    for (i, opt) in rest.split(',').filter(|s| !s.trim().is_empty()).enumerate() {
        let pos = i + 1;
        let mut kv = opt.splitn(2, '=');
        let key = kv
            .next()
            .ok_or_else(|| format!("malformed option at position {pos}: {opt}"))?
            .trim()
            .to_lowercase();
        let val = kv
            .next()
            .ok_or_else(|| format!("missing value for {key} at position {pos}"))?
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
                    .map_err(|e| format!("{key}={val} at position {pos}: invalid timeout: {e}"))?;
                module.timeout = if secs == 0 {
                    None
                } else {
                    Some(Duration::from_secs(secs))
                };
            }
            "use_chroot" => {
                module.use_chroot =
                    parse_bool(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?;
            }
            "numeric_ids" => {
                module.numeric_ids =
                    parse_bool(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?;
            }
            "uid" => {
                module.uid = Some(
                    parse_uid(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?,
                );
            }
            "gid" => {
                module.gid = Some(
                    parse_gid(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?,
                );
            }
            "read_only" => {
                module.read_only =
                    parse_bool(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?;
            }
            "write_only" => {
                module.write_only =
                    parse_bool(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?;
            }
            "list" => {
                module.list =
                    parse_bool(val).map_err(|e| format!("{key}={val} at position {pos}: {e}"))?;
            }
            "max_connections" => {
                let max = val.parse::<u32>().map_err(|e| {
                    format!("{key}={val} at position {pos}: invalid max connections: {e}")
                })?;
                module.max_connections = Some(max);
            }
            "refuse_options" => module.refuse_options = parse_list(val),
            _ => {
                return Err(format!("unknown option {key}={val} at position {pos}"));
            }
        }
    }

    Ok(module)
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
    validate_daemon_args(&opts)?;
    Ok(opts)
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
                validate_module(&m)?;
                cfg.modules.push(m);
            }
            let name = line[1..line.len() - 1].trim().to_string();
            current = Some(Module {
                name,
                path: PathBuf::new(),
                use_chroot: cfg.use_chroot.unwrap_or(true),
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
                cfg.port = Some(
                    val.parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                );
            }
            (false, "hosts allow") => cfg.hosts_allow = parse_list(&val),
            (false, "hosts deny") => cfg.hosts_deny = parse_list(&val),
            (false, "motd file") => cfg.motd_file = Some(PathBuf::from(val)),
            (false, "log file") => cfg.log_file = Some(PathBuf::from(val)),
            (false, "pid file") => cfg.pid_file = Some(PathBuf::from(val)),
            (false, "lock file") => cfg.lock_file = Some(PathBuf::from(val)),
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
                cfg.max_connections = Some(
                    val.parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                );
            }
            (false, "refuse options") => cfg.refuse_options = parse_list(&val),
            (true, "path") => {
                if let Some(ref mut m) = current {
                    m.path = fs::canonicalize(&val)?;
                }
            }
            (true, "hosts allow") => {
                if let Some(ref mut m) = current {
                    m.hosts_allow = parse_list(&val);
                }
            }
            (true, "hosts deny") => {
                if let Some(ref mut m) = current {
                    m.hosts_deny = parse_list(&val);
                }
            }
            (true, "auth users") => {
                if let Some(ref mut m) = current {
                    m.auth_users = parse_list(&val);
                }
            }
            (true, "comment") => {
                if let Some(ref mut m) = current {
                    m.comment = Some(val);
                }
            }
            (true, "secrets file") => {
                if let Some(ref mut m) = current {
                    m.secrets_file = Some(PathBuf::from(val));
                }
            }
            (true, "timeout") => {
                let secs = val
                    .parse::<u64>()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if let Some(ref mut m) = current {
                    m.timeout = if secs == 0 {
                        None
                    } else {
                        Some(Duration::from_secs(secs))
                    };
                }
            }
            (true, "use chroot") => {
                if let Some(ref mut m) = current {
                    m.use_chroot = parse_bool(&val)?;
                }
            }
            (true, "numeric ids") => {
                if let Some(ref mut m) = current {
                    m.numeric_ids = parse_bool(&val)?;
                }
            }
            (true, "uid") => {
                if let Some(ref mut m) = current {
                    m.uid = Some(parse_uid(&val)?);
                }
            }
            (true, "gid") => {
                if let Some(ref mut m) = current {
                    m.gid = Some(parse_gid(&val)?);
                }
            }
            (true, "read only") => {
                if let Some(ref mut m) = current {
                    m.read_only = parse_bool(&val)?;
                }
            }
            (true, "write only") => {
                if let Some(ref mut m) = current {
                    m.write_only = parse_bool(&val)?;
                }
            }
            (true, "list") => {
                if let Some(ref mut m) = current {
                    m.list = parse_bool(&val)?;
                }
            }
            (true, "max connections") => {
                if let Some(ref mut m) = current {
                    m.max_connections = Some(
                        val.parse()
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    );
                }
            }
            (true, "refuse options") => {
                if let Some(ref mut m) = current {
                    m.refuse_options = parse_list(&val);
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown option: {key}"),
                ));
            }
        }
    }
    if let Some(m) = current {
        validate_module(&m)?;
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
