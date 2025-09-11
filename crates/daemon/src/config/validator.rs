// crates/daemon/src/config/validator.rs

use std::io;
use transport::AddressFamily;

use super::model::{DaemonArgs, Module};

pub fn parse_bool(val: &str) -> io::Result<bool> {
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
pub fn parse_uid(val: &str) -> io::Result<u32> {
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
pub fn parse_uid(val: &str) -> io::Result<u32> {
    val.parse::<u32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(unix)]
pub fn parse_gid(val: &str) -> io::Result<u32> {
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
pub fn parse_gid(val: &str) -> io::Result<u32> {
    val.parse::<u32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn validate_daemon_args(opts: &DaemonArgs) -> io::Result<()> {
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
    Ok(())
}

pub fn validate_module(module: &Module) -> io::Result<()> {
    if module.path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("module {} missing path", module.name),
        ));
    }
    Ok(())
}
