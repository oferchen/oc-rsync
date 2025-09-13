// crates/daemon/src/service.rs
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::Ordering};
use std::time::{Duration, Instant};

#[cfg(unix)]
use crate::os::fork_daemon;
#[cfg(unix)]
use nix::unistd::{ForkResult, setsid};

use ipnet::IpNet;
use logging::{DebugFlag, InfoFlag, LogFormat, StderrMode, SubscriberConfig};
use protocol::{SUPPORTED_PROTOCOLS, negotiate_version};
#[cfg(unix)]
use sd_notify::{self, NotifyState};
use transport::{AddressFamily, RateLimitedTransport, TcpTransport, Transport};

use crate::auth::{authenticate, authenticate_token};
use crate::config::Module;

fn finish_session(transport: &mut dyn Transport) {
    let _ = transport.send(b"@RSYNCD: EXIT\n");
    let _ = transport.send(&[]);
    let _ = transport.close();
}

#[cfg(unix)]
pub struct PrivilegeContext {
    root: File,
    cwd: File,
    uid: u32,
    gid: u32,
    use_chroot: bool,
}

#[cfg(unix)]
impl Drop for PrivilegeContext {
    fn drop(&mut self) {
        use nix::unistd::{Gid, Uid, chroot, fchdir, setegid, seteuid};
        let _ = setegid(Gid::from_raw(self.gid));
        let _ = seteuid(Uid::from_raw(self.uid));
        if self.use_chroot {
            let _ = fchdir(&self.root);
            let _ = chroot(".");
        }
        let _ = fchdir(&self.cwd);
    }
}

#[cfg(not(unix))]
pub struct PrivilegeContext;

#[cfg(not(unix))]
impl Drop for PrivilegeContext {
    fn drop(&mut self) {}
}

pub fn init_logging(
    log_file: Option<&Path>,
    log_format: Option<&str>,
    syslog: bool,
    journald: bool,
    quiet: bool,
) -> io::Result<()> {
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
    logging::init(cfg)
}

#[cfg(unix)]
pub fn chroot_and_drop_privileges(
    path: &Path,
    uid: u32,
    gid: u32,
    use_chroot: bool,
) -> io::Result<PrivilegeContext> {
    use nix::unistd::{chdir, chroot, getegid, geteuid};
    let root_fd = File::open("/")?;
    let cwd_fd = File::open(".")?;
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
    Ok(PrivilegeContext {
        root: root_fd,
        cwd: cwd_fd,
        uid: euid,
        gid: egid,
        use_chroot,
    })
}

#[cfg(not(unix))]
pub fn chroot_and_drop_privileges(
    _path: &Path,
    _uid: u32,
    _gid: u32,
    _use_chroot: bool,
) -> io::Result<PrivilegeContext> {
    Ok(PrivilegeContext)
}

#[cfg(unix)]
pub fn drop_privileges(uid: u32, gid: u32) -> io::Result<()> {
    use nix::unistd::{Gid, Uid, setegid, seteuid};
    let cur_uid = Uid::current().as_raw();
    let cur_gid = Gid::current().as_raw();
    if gid != cur_gid {
        setegid(Gid::from_raw(gid)).map_err(io::Error::other)?;
    }
    if uid != cur_uid {
        seteuid(Uid::from_raw(uid)).map_err(io::Error::other)?;
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn drop_privileges(_uid: u32, _gid: u32) -> io::Result<()> {
    Ok(())
}

pub type Handler = dyn Fn(&mut dyn Transport, &[String]) -> io::Result<()> + Send + Sync;

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
pub fn handle_connection(
    transport: &mut dyn Transport,
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
    timeout: Option<Duration>,
) -> io::Result<()> {
    let mut log_file = log_file.map(|p| p.to_path_buf());
    let mut log_format = log_format.map(|s| s.to_string());
    let deadline = timeout.map(|d| Instant::now() + d);
    let res: io::Result<()> = (|| {
        let check_deadline = |t: &mut dyn Transport| -> io::Result<()> {
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    let _ = t.send(b"@ERROR: timeout waiting for daemon connection");
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timeout waiting for daemon connection",
                    ));
                }
            }
            Ok(())
        };

        check_deadline(transport)?;
        let mut buf = [0u8; 4];
        let n = match transport.receive(&mut buf) {
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                let _ = transport.send(b"@ERROR: timeout waiting for daemon connection");
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timeout waiting for daemon connection",
                ));
            }
            Err(e) => return Err(e),
        };
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
        check_deadline(transport)?;
        transport.send(b"@RSYNCD: OK\n")?;
        let mut name_buf = [0u8; 256];
        let n = transport.receive(&mut name_buf)?;
        let name = String::from_utf8_lossy(&name_buf[..n]).trim().to_string();
        if name.is_empty() || name == "#list" {
            if !list {
                let _ = transport.send(b"@ERROR: list denied");
            } else {
                for m in modules.values() {
                    if m.list {
                        let line = format!("{}\n", m.name);
                        transport.send(line.as_bytes())?;
                    }
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
            let mut opts = Vec::new();
            loop {
                check_deadline(transport)?;
                let n = match transport.receive(&mut opt_buf) {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        let _ = transport.send(b"@ERROR: timeout waiting for daemon connection");
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "timeout waiting for daemon connection",
                        ));
                    }
                    Err(e) => return Err(e),
                };
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
                let mut consumed = false;
                if let Some(v) = opt.strip_prefix("--log-file=") {
                    log_file = Some(PathBuf::from(v));
                    consumed = true;
                } else if let Some(v) = opt.strip_prefix("--log-file-format=") {
                    log_format = Some(v.to_string());
                    consumed = true;
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
                if !consumed {
                    opts.push(opt);
                }
            }
            if module.read_only && saw_server && !is_sender {
                let _ = transport.send(b"@ERROR: read only");
                if module.max_connections.is_some() {
                    module.connections.fetch_sub(1, Ordering::SeqCst);
                }
                finish_session(transport);
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "read only"));
            }
            if module.write_only && saw_server && is_sender {
                let _ = transport.send(b"@ERROR: write only");
                if module.max_connections.is_some() {
                    module.connections.fetch_sub(1, Ordering::SeqCst);
                }
                finish_session(transport);
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
            let (mut log, _guard) = serve_module(
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
            let res = handler(transport, &opts);
            let log_flush_res = if let Some(f) = log.as_mut() {
                f.flush()
            } else {
                Ok(())
            };
            let finish_res = {
                finish_session(transport);
                Ok(())
            };
            res.and(log_flush_res).and(finish_res)
        } else {
            let _ = transport.send(b"@ERROR: unknown module");
            finish_session(transport);
            Err(io::Error::new(io::ErrorKind::NotFound, "unknown module"))
        }
    })();
    res
}

pub fn serve_module(
    _t: &mut dyn Transport,
    module: &Module,
    peer: &str,
    log_file: Option<&Path>,
    log_format: Option<&str>,
    uid: u32,
    gid: u32,
) -> io::Result<(Option<File>, PrivilegeContext)> {
    let log = if let Some(path) = log_file {
        let fmt = log_format.unwrap_or("%h %m");
        let line = fmt.replace("%h", peer).replace("%m", &module.name);
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(f, "{}", line)?;
        Some(f)
    } else {
        None
    };
    let ctx = chroot_and_drop_privileges(&module.path, uid, gid, module.use_chroot)?;
    Ok((log, ctx))
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
    _max_connections: Option<usize>,
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
        match fork_daemon() {
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
    )?;

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

    let (listener, port) = TcpTransport::listen(address, port, family)?;
    let _ = writeln!(io::stdout(), "{port}");
    let _ = io::stdout().flush();
    #[cfg(unix)]
    let _ = sd_notify::notify(false, &[NotifyState::Ready]);

    loop {
        let (stream, addr) = TcpTransport::accept(&listener, &hosts_allow, &hosts_deny)?;
        let stream = TcpTransport::from_stream(stream);
        let mut transport: Box<dyn Transport> = if let Some(limit) = bwlimit {
            Box::new(RateLimitedTransport::new(stream, limit))
        } else {
            Box::new(stream)
        };
        if let Some(dur) = timeout {
            transport.set_read_timeout(Some(dur))?;
            transport.set_write_timeout(Some(dur))?;
        }
        let peer = addr.to_string();
        handle_connection(
            transport.as_mut(),
            &modules,
            secrets.as_deref(),
            password.as_deref(),
            log_file.as_deref(),
            log_format.as_deref(),
            motd.as_deref(),
            list,
            &refuse_options,
            &peer,
            uid,
            gid,
            &handler,
            timeout,
        )?;
    }
}
