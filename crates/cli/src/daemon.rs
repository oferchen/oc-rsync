// crates/cli/src/daemon.rs

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::net::IpAddr;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::utils::{parse_bool, parse_dparam};
use clap::{ArgMatches, Args};
use daemon::{parse_config_file, parse_module, Module};
use engine::{EngineError, Result, SyncOptions};
use logging::parse_escapes;
use protocol::{negotiate_version, CharsetConv, ExitCode};
use transport::{parse_sockopts, AddressFamily, SockOpt, TcpTransport, Transport};

#[derive(Args, Debug, Clone)]
pub struct DaemonOpts {
    #[arg(long)]
    pub daemon: bool,
    #[arg(long = "no-detach")]
    pub no_detach: bool,
    #[arg(long, value_parser = parse_module, value_name = "NAME=PATH")]
    module: Vec<Module>,
    #[arg(long)]
    pub address: Option<IpAddr>,
    #[arg(long = "secrets-file", value_name = "FILE")]
    pub secrets_file: Option<PathBuf>,
    #[arg(long = "hosts-allow", value_delimiter = ',', value_name = "LIST")]
    hosts_allow: Vec<String>,
    #[arg(long = "hosts-deny", value_delimiter = ',', value_name = "LIST")]
    hosts_deny: Vec<String>,
    #[arg(long = "motd", value_name = "FILE")]
    pub motd: Option<PathBuf>,
    #[arg(long = "pid-file", value_name = "FILE")]
    pub pid_file: Option<PathBuf>,
    #[arg(long = "lock-file", value_name = "FILE")]
    pub lock_file: Option<PathBuf>,
    #[arg(long = "state-dir", value_name = "DIR")]
    pub state_dir: Option<PathBuf>,
    #[arg(long = "dparam", value_name = "NAME=VALUE", value_parser = parse_dparam)]
    pub dparam: Vec<(String, String)>,
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_daemon_session(
    host: &str,
    module: &str,
    port: Option<u16>,
    password_file: Option<&Path>,
    no_motd: bool,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    family: Option<AddressFamily>,
    sockopts: &[String],
    opts: &SyncOptions,
    version: u32,
    early_input: Option<&Path>,
    iconv: Option<&CharsetConv>,
) -> Result<TcpTransport> {
    let (host, port) = if let Some((h, p)) = host.rsplit_once(':') {
        let p = p.parse().unwrap_or(873);
        (h, p)
    } else {
        (host, port.unwrap_or(873))
    };
    let start = Instant::now();
    let mut t =
        TcpTransport::connect(host, port, connect_timeout, family).map_err(EngineError::from)?;
    t.set_blocking_io(opts.blocking_io)
        .map_err(EngineError::from)?;
    let parsed: Vec<SockOpt> = parse_sockopts(sockopts).map_err(EngineError::Other)?;
    t.apply_sockopts(&parsed).map_err(EngineError::from)?;
    let handshake_timeout = connect_timeout
        .map(|dur| {
            dur.checked_sub(start.elapsed())
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))
        })
        .transpose()
        .map_err(EngineError::from)?;
    t.set_read_timeout(handshake_timeout)
        .map_err(EngineError::from)?;
    t.set_write_timeout(handshake_timeout)
        .map_err(EngineError::from)?;
    if let Some(p) = early_input {
        if let Ok(data) = fs::read(p) {
            t.send(&data).map_err(EngineError::from)?;
        }
    }
    t.send(&version.to_be_bytes()).map_err(EngineError::from)?;
    let mut buf = [0u8; 4];
    t.receive(&mut buf).map_err(EngineError::from)?;
    let peer = u32::from_be_bytes(buf);
    negotiate_version(version, peer).map_err(|e| EngineError::Other(e.to_string()))?;

    let token = password_file
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));
    t.authenticate(token.as_deref(), no_motd)
        .map_err(EngineError::from)?;

    let mut line = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = t.receive(&mut b).map_err(EngineError::from)?;
        if n == 0 {
            break;
        }
        line.push(b[0]);
        if b[0] == b'\n' {
            if line == b"@RSYNCD: OK\n" {
                break;
            }
            let s = if let Some(cv) = iconv {
                cv.decode_remote(&line)
            } else {
                String::from_utf8_lossy(&line).into_owned()
            };
            if let Some(err) = s.strip_prefix("@ERROR: ") {
                let msg = err.trim().to_string();
                if msg == "timeout waiting for daemon connection" {
                    return Err(EngineError::Exit(ExitCode::Timeout, msg));
                }
                return Err(EngineError::Other(msg));
            }
            if !no_motd {
                if let Some(msg) = s.strip_prefix("@RSYNCD: ") {
                    print!("{msg}");
                } else {
                    print!("{s}");
                }
                let _ = io::stdout().flush();
            }
            line.clear();
        }
    }
    t.set_read_timeout(timeout).map_err(EngineError::from)?;
    t.set_write_timeout(timeout).map_err(EngineError::from)?;

    if let Some(cv) = iconv {
        let mut line = cv.encode_remote(module);
        line.push(b'\n');
        t.send(&line).map_err(EngineError::from)?;
        for opt in &opts.remote_options {
            let mut o = cv.encode_remote(opt);
            o.push(b'\n');
            t.send(&o).map_err(EngineError::from)?;
        }
    } else {
        let line = format!("{module}\n");
        t.send(line.as_bytes()).map_err(EngineError::from)?;
        for opt in &opts.remote_options {
            let o = format!("{opt}\n");
            t.send(o.as_bytes()).map_err(EngineError::from)?;
        }
    }
    t.send(b"\n").map_err(EngineError::from)?;
    Ok(t)
}
pub(crate) fn run_daemon(opts: DaemonOpts, matches: &ArgMatches) -> Result<()> {
    let mut modules: HashMap<String, Module> = HashMap::new();
    let mut secrets = opts.secrets_file.clone();
    let password = matches
        .get_one::<PathBuf>("password_file")
        .cloned()
        .map(|pf| -> Result<String> {
            #[cfg(unix)]
            {
                let mode = fs::metadata(&pf)?.permissions().mode();
                if mode & 0o077 != 0 {
                    return Err(EngineError::Other(
                        "password file permissions are too open".into(),
                    ));
                }
            }
            let data = fs::read_to_string(&pf)?;
            Ok(data.lines().next().unwrap_or_default().trim().to_string())
        })
        .transpose()?;
    let mut hosts_allow = opts.hosts_allow.clone();
    let mut hosts_deny = opts.hosts_deny.clone();
    let mut log_file = matches.get_one::<PathBuf>("client-log-file").cloned();
    let log_format = matches
        .get_one::<String>("client-log-file-format")
        .map(|s| parse_escapes(s));
    let syslog = matches.get_flag("syslog");
    let journald = matches.get_flag("journald");
    let mut motd = opts.motd.clone();
    let mut pid_file = opts.pid_file.clone();
    let mut lock_file = opts.lock_file.clone();
    let mut state_dir = opts.state_dir.clone();
    let mut port = matches.get_one::<u16>("port").copied().unwrap_or(873);
    let mut address = opts.address;
    let timeout = matches.get_one::<Duration>("timeout").copied();
    let bwlimit = matches.get_one::<u64>("bwlimit").copied();
    let numeric_ids_flag = matches.get_flag("numeric_ids");
    let mut list = true;
    let mut refuse = Vec::new();
    let mut max_conn = None;
    let mut read_only = None;
    if let Some(cfg_path) = matches.get_one::<PathBuf>("config") {
        let cfg = parse_config_file(cfg_path).map_err(|e| EngineError::Other(e.to_string()))?;
        for m in cfg.modules {
            modules.insert(m.name.clone(), m);
        }
        if let Some(p) = cfg.port {
            port = p;
        }
        if let Some(m) = cfg.motd_file {
            motd = Some(m);
        }
        if let Some(l) = cfg.log_file {
            log_file = Some(l);
        }
        if let Some(s) = cfg.secrets_file {
            secrets = Some(s);
        }
        if let Some(p) = cfg.pid_file {
            pid_file = Some(p);
        }
        if let Some(l) = cfg.lock_file {
            lock_file = Some(l);
        }
        if let Some(a) = cfg.address {
            address = Some(a);
        }
        if !cfg.hosts_allow.is_empty() {
            hosts_allow = cfg.hosts_allow;
        }
        if !cfg.hosts_deny.is_empty() {
            hosts_deny = cfg.hosts_deny;
        }
        if let Some(val) = cfg.numeric_ids {
            for m in modules.values_mut() {
                m.numeric_ids = val;
            }
        }
        if let Some(val) = cfg.read_only {
            read_only = Some(val);
        }
        if let Some(val) = cfg.list {
            list = val;
        }
        if let Some(val) = cfg.max_connections {
            max_conn = Some(val);
        }
        if !cfg.refuse_options.is_empty() {
            refuse = cfg.refuse_options;
        }
    }

    for m in opts.module {
        modules.insert(m.name.clone(), m);
    }

    for (name, value) in opts.dparam {
        match name.as_str() {
            "motdfile" => motd = Some(PathBuf::from(value)),
            "pidfile" => pid_file = Some(PathBuf::from(value)),
            "logfile" => log_file = Some(PathBuf::from(value)),
            "lockfile" => lock_file = Some(PathBuf::from(value)),
            "statedir" => state_dir = Some(PathBuf::from(value)),
            "secretsfile" => secrets = Some(PathBuf::from(value)),
            "address" => {
                address = Some(
                    value
                        .parse::<IpAddr>()
                        .map_err(|e| EngineError::Other(e.to_string()))?,
                )
            }
            "port" => {
                port = value
                    .parse::<u16>()
                    .map_err(|e| EngineError::Other(e.to_string()))?
            }
            "numericids" => {
                let val = parse_bool(&value).map_err(EngineError::Other)?;
                for m in modules.values_mut() {
                    m.numeric_ids = val;
                }
            }
            "read only" | "read_only" => {
                let val = parse_bool(&value).map_err(EngineError::Other)?;
                for m in modules.values_mut() {
                    m.read_only = val;
                }
            }
            "list" => {
                list = parse_bool(&value).map_err(EngineError::Other)?;
            }
            "max connections" | "maxconnections" => {
                max_conn = Some(
                    value
                        .parse::<usize>()
                        .map_err(|e| EngineError::Other(e.to_string()))?,
                );
            }
            "hosts allow" | "hostsallow" => {
                hosts_allow = value.split_whitespace().map(|s| s.to_string()).collect();
            }
            "hosts deny" | "hostsdeny" => {
                hosts_deny = value.split_whitespace().map(|s| s.to_string()).collect();
            }
            "refuse options" | "refuseoptions" => {
                refuse = value.split_whitespace().map(|s| s.to_string()).collect();
            }
            _ => {
                return Err(EngineError::Other(format!(
                    "unknown daemon parameter: {name}"
                )));
            }
        }
    }

    if numeric_ids_flag {
        for m in modules.values_mut() {
            m.numeric_ids = true;
        }
    }
    if let Some(val) = read_only {
        for m in modules.values_mut() {
            m.read_only = val;
        }
    }
    if !refuse.is_empty() {
        for m in modules.values_mut() {
            m.refuse_options = refuse.clone();
        }
    }

    let addr_family = if matches.get_flag("ipv4") {
        Some(AddressFamily::V4)
    } else if matches.get_flag("ipv6") {
        Some(AddressFamily::V6)
    } else {
        None
    };

    let handler: Arc<daemon::Handler> = Arc::new(|_| Ok(()));
    let quiet = matches.get_flag("quiet");

    daemon::run_daemon(
        modules,
        secrets,
        password,
        hosts_allow,
        hosts_deny,
        log_file,
        log_format,
        syslog,
        journald,
        motd,
        pid_file,
        lock_file,
        state_dir,
        timeout,
        bwlimit,
        max_conn,
        refuse,
        list,
        port,
        address,
        addr_family,
        65534,
        65534,
        handler,
        quiet,
        opts.no_detach,
    )
    .map_err(|e| EngineError::Other(format!("daemon failed to bind to port {port}: {e}")))
}
