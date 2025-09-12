// crates/cli/src/exec/transfer/remote_remote.rs

use std::path::Path;

use crate::options::ClientOpts;
use crate::session::check_session_errors;
use crate::utils::{PathSpec, RshCommand};
use crate::{EngineError, spawn_daemon_session};
use oc_rsync_core::{
    message::CharsetConv,
    transfer::{Result, Stats, SyncOptions, pipe_sessions},
    transport::{AddressFamily, RateLimitedTransport, SshStdioTransport, daemon_remote_opts},
};

#[allow(clippy::too_many_arguments)]
pub(super) fn remote_to_remote(
    src_host: String,
    src_path: PathSpec,
    src_module: Option<String>,
    dst_host: String,
    dst_path: PathSpec,
    dst_module: Option<String>,
    opts: &ClientOpts,
    rsh_cmd: &RshCommand,
    remote_bin: Option<&[String]>,
    remote_env: Option<&[(String, String)]>,
    known_hosts: Option<&Path>,
    strict_host_key_checking: bool,
    addr_family: Option<AddressFamily>,
    iconv: Option<&CharsetConv>,
    sync_opts: &mut SyncOptions,
) -> Result<Stats> {
    match (src_module, dst_module) {
        (None, None) => {
            let connect_timeout = opts.connect_timeout;
            let mut dst_session = SshStdioTransport::spawn_with_rsh(
                &dst_host,
                &dst_path.path,
                &rsh_cmd.cmd,
                &rsh_cmd.env,
                remote_bin,
                remote_env.unwrap_or(&[]),
                &sync_opts.remote_options,
                known_hosts,
                strict_host_key_checking,
                opts.port,
                connect_timeout,
                addr_family,
                sync_opts.blocking_io,
            )
            .map_err(EngineError::from)?;
            let mut src_session = SshStdioTransport::spawn_with_rsh(
                &src_host,
                &src_path.path,
                &rsh_cmd.cmd,
                &rsh_cmd.env,
                remote_bin,
                remote_env.unwrap_or(&[]),
                &sync_opts.remote_options,
                known_hosts,
                strict_host_key_checking,
                opts.port,
                connect_timeout,
                addr_family,
                sync_opts.blocking_io,
            )
            .map_err(EngineError::from)?;
            if let Some(limit) = opts.bwlimit {
                let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                check_session_errors(&src_session, iconv)?;
                let dst_session = dst_session.into_inner();
                check_session_errors(&dst_session, iconv)?;
                Ok(stats)
            } else {
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                check_session_errors(&src_session, iconv)?;
                check_session_errors(&dst_session, iconv)?;
                Ok(stats)
            }
        }
        (Some(sm), Some(dm)) => {
            let mut src_opts = sync_opts.clone();
            src_opts.remote_options = daemon_remote_opts(&sync_opts.remote_options, &src_path.path);
            let mut dst_opts = sync_opts.clone();
            dst_opts.remote_options = daemon_remote_opts(&sync_opts.remote_options, &dst_path.path);
            let mut src_session = spawn_daemon_session(
                &src_host,
                &sm,
                opts.port,
                opts.password_file.as_deref(),
                opts.no_motd,
                opts.timeout,
                opts.connect_timeout,
                addr_family,
                &opts.sockopts,
                &src_opts,
                opts.protocol.unwrap_or(31),
                opts.early_input.as_deref(),
                iconv,
            )?;
            let mut dst_session = spawn_daemon_session(
                &dst_host,
                &dm,
                opts.port,
                opts.password_file.as_deref(),
                opts.no_motd,
                opts.timeout,
                opts.connect_timeout,
                addr_family,
                &opts.sockopts,
                &dst_opts,
                opts.protocol.unwrap_or(31),
                opts.early_input.as_deref(),
                iconv,
            )?;
            if let Some(limit) = opts.bwlimit {
                let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                Ok(stats)
            } else {
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                Ok(stats)
            }
        }
        (Some(sm), None) => {
            let mut dst_session = SshStdioTransport::spawn_with_rsh(
                &dst_host,
                &dst_path.path,
                &rsh_cmd.cmd,
                &rsh_cmd.env,
                remote_bin,
                remote_env.unwrap_or(&[]),
                &sync_opts.remote_options,
                known_hosts,
                strict_host_key_checking,
                opts.port,
                opts.connect_timeout,
                addr_family,
                sync_opts.blocking_io,
            )
            .map_err(EngineError::from)?;
            let mut src_opts = sync_opts.clone();
            src_opts.remote_options = daemon_remote_opts(&sync_opts.remote_options, &src_path.path);
            let mut src_session = spawn_daemon_session(
                &src_host,
                &sm,
                opts.port,
                opts.password_file.as_deref(),
                opts.no_motd,
                opts.timeout,
                opts.connect_timeout,
                addr_family,
                &opts.sockopts,
                &src_opts,
                opts.protocol.unwrap_or(31),
                opts.early_input.as_deref(),
                iconv,
            )?;
            if let Some(limit) = opts.bwlimit {
                let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                let dst_session = dst_session.into_inner();
                check_session_errors(&dst_session, iconv)?;
                Ok(stats)
            } else {
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                check_session_errors(&dst_session, iconv)?;
                Ok(stats)
            }
        }
        (None, Some(dm)) => {
            let mut dst_opts = sync_opts.clone();
            dst_opts.remote_options = daemon_remote_opts(&sync_opts.remote_options, &dst_path.path);
            let mut dst_session = spawn_daemon_session(
                &dst_host,
                &dm,
                opts.port,
                opts.password_file.as_deref(),
                opts.no_motd,
                opts.timeout,
                opts.connect_timeout,
                addr_family,
                &opts.sockopts,
                &dst_opts,
                opts.protocol.unwrap_or(31),
                opts.early_input.as_deref(),
                iconv,
            )?;
            let mut src_session = SshStdioTransport::spawn_with_rsh(
                &src_host,
                &src_path.path,
                &rsh_cmd.cmd,
                &rsh_cmd.env,
                remote_bin,
                remote_env.unwrap_or(&[]),
                &sync_opts.remote_options,
                known_hosts,
                strict_host_key_checking,
                opts.port,
                opts.connect_timeout,
                addr_family,
                sync_opts.blocking_io,
            )
            .map_err(EngineError::from)?;
            if let Some(limit) = opts.bwlimit {
                let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                check_session_errors(&src_session, iconv)?;
                Ok(stats)
            } else {
                let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                check_session_errors(&src_session, iconv)?;
                Ok(stats)
            }
        }
    }
}
