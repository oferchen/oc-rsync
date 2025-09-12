// crates/transport/src/ssh/spawn.rs

use std::ffi::OsStr;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use compress::Codec;

use crate::{AddressFamily, LocalPipeTransport, Transport};

use super::session::{
    CapturedStderr, ProcessHandle, SSH_IO_BUF_SIZE, SSH_STDERR_CAP, SshStdioTransport,
};

impl SshStdioTransport {
    pub fn spawn<I, S>(program: &str, args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new(program);
        cmd.args(args);
        Self::spawn_from_command(cmd, false)
    }

    pub fn spawn_server<I, S>(
        host: &str,
        server_args: I,
        remote_opts: &[String],
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
        port: Option<u16>,
        family: Option<AddressFamily>,
    ) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut cmd = Command::new("ssh");

        let known_hosts_path = known_hosts.map(Path::to_path_buf).or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".ssh/known_hosts"))
        });

        let checking = if strict_host_key_checking {
            "yes"
        } else {
            "no"
        };
        cmd.arg("-o")
            .arg(format!("StrictHostKeyChecking={checking}"));
        if let Some(path) = known_hosts_path {
            cmd.arg("-o")
                .arg(format!("UserKnownHostsFile={}", path.display()));
        }
        if let Some(p) = port {
            cmd.arg("-p").arg(p.to_string());
        }
        if let Some(AddressFamily::V4) = family {
            cmd.arg("-4");
        } else if let Some(AddressFamily::V6) = family {
            cmd.arg("-6");
        }
        cmd.arg(host);
        cmd.arg("rsync");
        cmd.arg("--server");
        cmd.args(remote_opts);
        cmd.args(server_args);

        Self::spawn_from_command(cmd, false)
    }

    pub fn spawn_from_command(mut cmd: Command, blocking_io: bool) -> io::Result<Self> {
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("missing stderr"))?;

        let stderr_buf = Arc::new(Mutex::new(CapturedStderr::default()));
        let stderr_buf_clone = Arc::clone(&stderr_buf);
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut buf = Vec::new();
            let mut chunk = [0u8; SSH_IO_BUF_SIZE];
            let mut truncated = false;
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if buf.len() < SSH_STDERR_CAP {
                            let remaining = SSH_STDERR_CAP - buf.len();
                            let take = remaining.min(n);
                            buf.extend_from_slice(&chunk[..take]);
                            if n > take {
                                truncated = true;
                            }
                        } else {
                            truncated = true;
                        }
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut locked) = stderr_buf_clone.lock() {
                locked.data = buf;
                locked.truncated = truncated;
            }
        });

        let handle = ProcessHandle {
            child,
            stderr_thread: Some(stderr_thread),
        };

        let mut t = Self {
            inner: Some(LocalPipeTransport::new(
                BufReader::with_capacity(SSH_IO_BUF_SIZE, stdout),
                stdin,
            )),
            stderr: stderr_buf,
            handle: Some(handle),
            read_timeout: None,
            write_timeout: None,
            blocking_io: false,
        };
        t.set_blocking_io(blocking_io)?;
        Ok(t)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_with_rsh(
        host: &str,
        path: &Path,
        rsh: &[String],
        rsh_env: &[(String, String)],
        remote_bin: Option<&[String]>,
        remote_bin_env: &[(String, String)],
        remote_opts: &[String],
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
        port: Option<u16>,
        connect_timeout: Option<Duration>,
        family: Option<AddressFamily>,
        blocking_io: bool,
    ) -> io::Result<Self> {
        let program = rsh.first().map(|s| s.as_str()).unwrap_or("ssh");
        if program == "ssh" {
            let mut cmd = Command::new(program);
            cmd.envs(rsh_env.iter().cloned());
            cmd.args(&rsh[1..]);
            let known_hosts_path = known_hosts.map(Path::to_path_buf).or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".ssh/known_hosts"))
            });
            let checking = if strict_host_key_checking {
                "yes"
            } else {
                "no"
            };
            cmd.arg("-o")
                .arg(format!("StrictHostKeyChecking={checking}"));
            if let Some(path) = known_hosts_path {
                cmd.arg("-o")
                    .arg(format!("UserKnownHostsFile={}", path.display()));
            }
            if let Some(dur) = connect_timeout {
                cmd.arg("-o")
                    .arg(format!("ConnectTimeout={}", dur.as_secs()));
            }
            if let Some(p) = port {
                cmd.arg("-p").arg(p.to_string());
            }
            if let Some(AddressFamily::V4) = family {
                cmd.arg("-4");
            } else if let Some(AddressFamily::V6) = family {
                cmd.arg("-6");
            }
            cmd.arg(host);
            if let Some(bin) = remote_bin {
                for (k, v) in remote_bin_env {
                    cmd.arg(format!("{k}={v}"));
                }
                cmd.args(bin);
            } else {
                cmd.arg("rsync");
            }
            cmd.arg("--server");
            cmd.args(remote_opts);
            cmd.arg(path.as_os_str());
            Self::spawn_from_command(cmd, blocking_io)
        } else {
            let mut args = rsh[1..].to_vec();
            let host = if let Some(p) = port {
                format!("{host}:{p}")
            } else {
                host.to_string()
            };
            args.push(host);
            if let Some(bin) = remote_bin {
                for (k, v) in remote_bin_env {
                    args.push(format!("{k}={v}"));
                }
                args.extend_from_slice(bin);
            } else {
                args.push("rsync".to_string());
            }
            args.push("--server".to_string());
            args.extend_from_slice(remote_opts);
            args.push(path.to_string_lossy().into_owned());
            let mut cmd = Command::new(program);
            cmd.envs(rsh_env.iter().cloned());
            cmd.args(args);
            Self::spawn_from_command(cmd, blocking_io)
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn connect_with_rsh(
        host: &str,
        path: &Path,
        rsh: &[String],
        rsh_env: &[(String, String)],
        rsync_env: &[(String, String)],
        remote_bin: Option<&[String]>,
        remote_bin_env: &[(String, String)],
        remote_opts: &[String],
        known_hosts: Option<&Path>,
        strict_host_key_checking: bool,
        port: Option<u16>,
        connect_timeout: Option<Duration>,
        family: Option<AddressFamily>,
        blocking_io: bool,
        version: u32,
        caps: u32,
        token: Option<&str>,
    ) -> io::Result<(Self, Vec<Codec>, u32)> {
        let start = Instant::now();
        let mut t = Self::spawn_with_rsh(
            host,
            path,
            rsh,
            rsh_env,
            remote_bin,
            remote_bin_env,
            remote_opts,
            known_hosts,
            strict_host_key_checking,
            port,
            connect_timeout,
            family,
            blocking_io,
        )?;
        if let Some(dur) = connect_timeout {
            let elapsed = start.elapsed();
            let remaining = dur
                .checked_sub(elapsed)
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))?;
            t.set_read_timeout(Some(remaining))?;
            t.set_write_timeout(Some(remaining))?;
        }
        let (codecs, caps) =
            match Self::handshake(&mut t, rsync_env, remote_opts, token, version, caps) {
                Ok(v) => v,
                Err(mut e) => {
                    let (stderr, _) = t.stderr();
                    if !stderr.is_empty() {
                        let mut msg = String::from_utf8_lossy(&stderr).into_owned();
                        if !msg.ends_with('\n') {
                            msg.push('\n');
                        }
                        msg.push_str("connection unexpectedly closed");
                        let kind = if msg.contains("Permission denied") {
                            io::ErrorKind::PermissionDenied
                        } else if msg.contains("Connection refused") {
                            io::ErrorKind::ConnectionRefused
                        } else if msg.contains("Connection timed out") {
                            io::ErrorKind::TimedOut
                        } else if msg.contains("No route to host") {
                            io::ErrorKind::HostUnreachable
                        } else if msg.contains("Name or service not known")
                            || msg.contains("Temporary failure in name resolution")
                        {
                            io::ErrorKind::NotFound
                        } else {
                            io::ErrorKind::UnexpectedEof
                        };
                        e = io::Error::new(kind, msg);
                    }
                    return Err(e);
                }
            };
        if connect_timeout.is_some() {
            t.set_read_timeout(None)?;
            t.set_write_timeout(None)?;
        }
        Ok((t, codecs, caps))
    }
}
