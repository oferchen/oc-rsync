// crates/transport/src/daemon.rs
use std::path::Path;
use std::time::Duration;

use crate::Transport;

pub trait DaemonTransport: Transport {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SockOpt {
    KeepAlive(bool),
    SendBuf(usize),
    RecvBuf(usize),
    IpTtl(u32),
    IpTos(u32),
    TcpNoDelay(bool),
    ReuseAddr(bool),
    BindToDevice(String),
    IpHopLimit(u32),
    Linger(Option<Duration>),
    Broadcast(bool),
    RcvTimeout(Duration),
    SndTimeout(Duration),
}

pub fn parse_sockopts(opts: &[String]) -> Result<Vec<SockOpt>, String> {
    opts.iter().map(|s| parse_sockopt(s)).collect()
}

fn parse_sockopt(s: &str) -> Result<SockOpt, String> {
    if let Some((prefix, rest)) = s.split_once(':') {
        return parse_prefixed_sockopt(prefix, rest);
    }

    let (name, value) = match s.split_once('=') {
        Some((n, v)) => (n.trim(), Some(v.trim())),
        None => (s.trim(), None),
    };
    match name {
        "SO_KEEPALIVE" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::KeepAlive(enabled))
        }
        "SO_SNDBUF" => {
            let v = value.ok_or_else(|| "SO_SNDBUF requires a value".to_string())?;
            let size = v
                .parse::<usize>()
                .map_err(|_| "invalid SO_SNDBUF value".to_string())?;
            Ok(SockOpt::SendBuf(size))
        }
        "SO_RCVBUF" => {
            let v = value.ok_or_else(|| "SO_RCVBUF requires a value".to_string())?;
            let size = v
                .parse::<usize>()
                .map_err(|_| "invalid SO_RCVBUF value".to_string())?;
            Ok(SockOpt::RecvBuf(size))
        }
        "TCP_NODELAY" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::TcpNoDelay(enabled))
        }
        "SO_REUSEADDR" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::ReuseAddr(enabled))
        }
        "SO_BINDTODEVICE" => {
            let v = value.ok_or_else(|| "SO_BINDTODEVICE requires a value".to_string())?;
            if v.is_empty() {
                return Err("SO_BINDTODEVICE requires a non-empty value".to_string());
            }
            Ok(SockOpt::BindToDevice(v.to_string()))
        }
        "SO_LINGER" => {
            let dur = value
                .map(|v| parse_u64(v).map(Duration::from_secs))
                .transpose()?;
            Ok(SockOpt::Linger(dur))
        }
        "SO_BROADCAST" => {
            let enabled = value.map(|v| v != "0").unwrap_or(true);
            Ok(SockOpt::Broadcast(enabled))
        }
        "SO_RCVTIMEO" => {
            let v = value.ok_or_else(|| "SO_RCVTIMEO requires a value".to_string())?;
            let secs = parse_u64(v)?;
            Ok(SockOpt::RcvTimeout(Duration::from_secs(secs)))
        }
        "SO_SNDTIMEO" => {
            let v = value.ok_or_else(|| "SO_SNDTIMEO requires a value".to_string())?;
            let secs = parse_u64(v)?;
            Ok(SockOpt::SndTimeout(Duration::from_secs(secs)))
        }
        _ => Err(format!("unknown socket option: {name}")),
    }
}

fn parse_prefixed_sockopt(prefix: &str, rest: &str) -> Result<SockOpt, String> {
    match prefix.to_ascii_lowercase().as_str() {
        "ip" => {
            let (name, value) = rest
                .split_once('=')
                .ok_or_else(|| "ip option requires a value".to_string())?;
            let val = parse_u32(value)?;
            match name.to_ascii_lowercase().as_str() {
                "ttl" => Ok(SockOpt::IpTtl(val)),
                "tos" => Ok(SockOpt::IpTos(val)),
                "hoplimit" => Ok(SockOpt::IpHopLimit(val)),
                _ => Err(format!("unknown ip socket option: {name}")),
            }
        }
        _ => Err(format!("unknown socket option: {prefix}:{rest}")),
    }
}

fn parse_u32(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).map_err(|_| "invalid numeric value".to_string())
    } else {
        s.parse::<u32>()
            .map_err(|_| "invalid numeric value".to_string())
    }
}

fn parse_u64(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).map_err(|_| "invalid numeric value".to_string())
    } else {
        s.parse::<u64>()
            .map_err(|_| "invalid numeric value".to_string())
    }
}

pub fn daemon_remote_opts(base: &[String], path: &Path) -> Vec<String> {
    let mut opts = base.to_vec();
    if path != Path::new(".") {
        opts.push(path.to_string_lossy().into_owned());
    }
    opts
}
