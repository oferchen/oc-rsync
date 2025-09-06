// crates/cli/src/exec.rs

use std::path::Path;

use clap::{ArgMatches, parser::ValueSource};

use crate::options::ClientOpts;
use crate::session::check_session_errors;
use crate::utils::{RemoteSpec, RshCommand};
use crate::{EngineError, spawn_daemon_session};

use compress::available_codecs;
use engine::{Result, Stats, SyncOptions, pipe_sessions, sync};
use filters::Matcher;
use protocol::{CAP_ACLS, CAP_CODECS, CAP_XATTRS, CharsetConv, ExitCode};
use transport::{AddressFamily, RateLimitedTransport, SshStdioTransport, daemon_remote_opts};

#[cfg(unix)]
use nix::unistd;

#[cfg(target_os = "linux")]
use caps::{self, CapSet, Capability};

pub(crate) fn check_privileges(opts: &mut ClientOpts, matches: &ArgMatches) -> Result<()> {
    #[cfg(unix)]
    {
        let need_owner = if opts.no_owner {
            false
        } else {
            opts.owner || opts.archive
        };
        let need_group = if opts.no_group {
            false
        } else {
            opts.group || opts.archive
        };
        let maps_requested =
            opts.chown.is_some() || !opts.usermap.is_empty() || !opts.groupmap.is_empty();
        let needs_privs = need_owner || need_group || maps_requested;
        let numeric_fallback = opts.numeric_ids
            && opts.chown.is_none()
            && opts.usermap.is_empty()
            && opts.groupmap.is_empty();
        if needs_privs && !numeric_fallback && !is_effective_root() {
            #[cfg(target_os = "linux")]
            let has_privs = match has_cap_chown() {
                Ok(v) => v,
                Err(e) => {
                    return Err(EngineError::Other(format!(
                        "failed to detect CAP_CHOWN capability: {e}"
                    )));
                }
            };
            #[cfg(not(target_os = "linux"))]
            let has_privs = false;

            let priv_msg = if cfg!(target_os = "linux") {
                "changing ownership requires root or CAP_CHOWN"
            } else {
                "changing ownership requires root"
            };

            if !has_privs {
                if maps_requested {
                    return Err(EngineError::Exit(ExitCode::StartClient, priv_msg.into()));
                }
                let owner_explicit =
                    matches.value_source("owner") == Some(ValueSource::CommandLine);
                let group_explicit =
                    matches.value_source("group") == Some(ValueSource::CommandLine);
                let mut downgraded = false;
                if need_owner && !owner_explicit {
                    opts.owner = false;
                    opts.no_owner = true;
                    downgraded = true;
                }
                if need_group && !group_explicit {
                    opts.group = false;
                    opts.no_group = true;
                    downgraded = true;
                }
                if downgraded {
                    tracing::warn!("{priv_msg}: disabling owner/group");
                } else {
                    return Err(EngineError::Exit(ExitCode::StartClient, priv_msg.into()));
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_transfer(
    src: RemoteSpec,
    dst: RemoteSpec,
    matcher: &Matcher,
    opts: &ClientOpts,
    rsh_cmd: &RshCommand,
    rsync_env: &[(String, String)],
    remote_bin: Option<&[String]>,
    remote_env: Option<&[(String, String)]>,
    known_hosts: Option<&Path>,
    strict_host_key_checking: bool,
    addr_family: Option<AddressFamily>,
    iconv: Option<&CharsetConv>,
    sync_opts: &mut SyncOptions,
) -> Result<Stats> {
    sync_opts.prepare_remote();
    let local = matches!(&src, RemoteSpec::Local(_)) && matches!(&dst, RemoteSpec::Local(_));
    let stats = if local {
        match (src, dst) {
            (RemoteSpec::Local(src), RemoteSpec::Local(dst)) => sync(
                &src.path,
                &dst.path,
                matcher,
                &available_codecs(),
                sync_opts,
            )?,
            _ => unreachable!(),
        }
    } else {
        match (src, dst) {
            (RemoteSpec::Local(_), RemoteSpec::Local(_)) => unreachable!(),
            (
                RemoteSpec::Remote {
                    host,
                    path: src,
                    module: Some(module),
                },
                RemoteSpec::Local(dst),
            ) => {
                let mut _session = spawn_daemon_session(
                    &host,
                    &module,
                    opts.port,
                    opts.password_file.as_deref(),
                    opts.no_motd,
                    opts.timeout,
                    opts.connect_timeout,
                    addr_family,
                    &opts.sockopts,
                    sync_opts,
                    opts.protocol.unwrap_or(31),
                    opts.early_input.as_deref(),
                    iconv,
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    matcher,
                    &available_codecs(),
                    sync_opts,
                )?
            }
            (
                RemoteSpec::Remote {
                    host,
                    path: src,
                    module: None,
                },
                RemoteSpec::Local(dst),
            ) => {
                let connect_timeout = opts.connect_timeout;
                let caps_send = CAP_CODECS
                    | if sync_opts.acls { CAP_ACLS } else { 0 }
                    | if sync_opts.xattrs { CAP_XATTRS } else { 0 };
                let (session, codecs, caps) = SshStdioTransport::connect_with_rsh(
                    &host,
                    &src.path,
                    &rsh_cmd.cmd,
                    &rsh_cmd.env,
                    rsync_env,
                    remote_bin,
                    remote_env.unwrap_or(&[]),
                    &sync_opts.remote_options,
                    known_hosts,
                    strict_host_key_checking,
                    opts.port,
                    connect_timeout,
                    addr_family,
                    sync_opts.blocking_io,
                    opts.protocol.unwrap_or(31),
                    caps_send,
                    None,
                )
                .map_err(EngineError::from)?;
                if sync_opts.xattrs && caps & CAP_XATTRS == 0 {
                    sync_opts.xattrs = false;
                }
                if sync_opts.acls && caps & CAP_ACLS == 0 {
                    sync_opts.acls = false;
                }
                let (err, _) = session.stderr();
                if !err.is_empty() {
                    let msg = if let Some(cv) = iconv {
                        cv.decode_remote(&err).into_owned()
                    } else {
                        String::from_utf8_lossy(&err).into_owned()
                    };
                    return Err(EngineError::Other(msg));
                }
                sync(&src.path, &dst.path, matcher, &codecs, sync_opts)?
            }
            (
                RemoteSpec::Local(src),
                RemoteSpec::Remote {
                    host,
                    path: dst,
                    module: Some(module),
                },
            ) => {
                let mut _session = spawn_daemon_session(
                    &host,
                    &module,
                    opts.port,
                    opts.password_file.as_deref(),
                    opts.no_motd,
                    opts.timeout,
                    opts.connect_timeout,
                    addr_family,
                    &opts.sockopts,
                    sync_opts,
                    opts.protocol.unwrap_or(31),
                    opts.early_input.as_deref(),
                    iconv,
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    matcher,
                    &available_codecs(),
                    sync_opts,
                )?
            }
            (
                RemoteSpec::Local(src),
                RemoteSpec::Remote {
                    host,
                    path: dst,
                    module: None,
                },
            ) => {
                let connect_timeout = opts.connect_timeout;
                let caps_send = CAP_CODECS
                    | if sync_opts.acls { CAP_ACLS } else { 0 }
                    | if sync_opts.xattrs { CAP_XATTRS } else { 0 };
                let (session, codecs, caps) = SshStdioTransport::connect_with_rsh(
                    &host,
                    &dst.path,
                    &rsh_cmd.cmd,
                    &rsh_cmd.env,
                    rsync_env,
                    remote_bin,
                    remote_env.unwrap_or(&[]),
                    &sync_opts.remote_options,
                    known_hosts,
                    strict_host_key_checking,
                    opts.port,
                    connect_timeout,
                    addr_family,
                    sync_opts.blocking_io,
                    opts.protocol.unwrap_or(31),
                    caps_send,
                    None,
                )
                .map_err(EngineError::from)?;
                if sync_opts.xattrs && caps & CAP_XATTRS == 0 {
                    sync_opts.xattrs = false;
                }
                if sync_opts.acls && caps & CAP_ACLS == 0 {
                    sync_opts.acls = false;
                }
                let (err, _) = session.stderr();
                if !err.is_empty() {
                    let msg = if let Some(cv) = iconv {
                        cv.decode_remote(&err).into_owned()
                    } else {
                        String::from_utf8_lossy(&err).into_owned()
                    };
                    return Err(EngineError::Other(msg));
                }
                sync(&src.path, &dst.path, matcher, &codecs, sync_opts)?
            }
            (
                RemoteSpec::Remote {
                    host: src_host,
                    path: src_path,
                    module: src_module,
                },
                RemoteSpec::Remote {
                    host: dst_host,
                    path: dst_path,
                    module: dst_module,
                },
            ) => match (src_module, dst_module) {
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
                        stats
                    } else {
                        let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                        check_session_errors(&src_session, iconv)?;
                        check_session_errors(&dst_session, iconv)?;
                        stats
                    }
                }
                (Some(sm), Some(dm)) => {
                    let mut src_opts = sync_opts.clone();
                    src_opts.remote_options =
                        daemon_remote_opts(&sync_opts.remote_options, &src_path.path);
                    let mut dst_opts = sync_opts.clone();
                    dst_opts.remote_options =
                        daemon_remote_opts(&sync_opts.remote_options, &dst_path.path);
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
                        pipe_sessions(&mut src_session, &mut dst_session)?
                    } else {
                        pipe_sessions(&mut src_session, &mut dst_session)?
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
                    src_opts.remote_options =
                        daemon_remote_opts(&sync_opts.remote_options, &src_path.path);
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
                        stats
                    } else {
                        let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                        check_session_errors(&dst_session, iconv)?;
                        stats
                    }
                }
                (None, Some(dm)) => {
                    let mut dst_opts = sync_opts.clone();
                    dst_opts.remote_options =
                        daemon_remote_opts(&sync_opts.remote_options, &dst_path.path);
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
                        stats
                    } else {
                        let stats = pipe_sessions(&mut src_session, &mut dst_session)?;
                        check_session_errors(&src_session, iconv)?;
                        stats
                    }
                }
            },
        }
    };
    Ok(stats)
}

#[cfg(unix)]
fn is_effective_root() -> bool {
    #[cfg(test)]
    if let Some(v) = MOCK_IS_ROOT.with(|m| m.borrow_mut().take()) {
        return v;
    }
    unistd::Uid::effective().is_root()
}

#[cfg(all(test, unix))]
thread_local! {
    static MOCK_IS_ROOT: std::cell::RefCell<Option<bool>> = std::cell::RefCell::new(None);
}

#[cfg(all(test, unix))]
fn mock_effective_root(val: bool) {
    MOCK_IS_ROOT.with(|m| *m.borrow_mut() = Some(val));
}

#[cfg(target_os = "linux")]
fn has_cap_chown() -> std::result::Result<bool, caps::errors::CapsError> {
    #[cfg(test)]
    if let Some(res) = MOCK_CAPS.with(|m| m.borrow_mut().take()) {
        return res;
    }
    caps::has_cap(None, CapSet::Effective, Capability::CAP_CHOWN)
}

#[cfg(all(test, target_os = "linux"))]
thread_local! {
    static MOCK_CAPS: std::cell::RefCell<
        Option<std::result::Result<bool, caps::errors::CapsError>>,
    > = std::cell::RefCell::new(None);
}

#[cfg(all(test, target_os = "linux"))]
fn mock_caps_has_cap(res: std::result::Result<bool, caps::errors::CapsError>) {
    MOCK_CAPS.with(|m| *m.borrow_mut() = Some(res));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::cli_command;
    use crate::utils::PathSpec;
    use clap::FromArgMatches;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn privilege_downgrade_without_root() {
        let cmd = cli_command();
        let matches = cmd
            .clone()
            .try_get_matches_from(["prog", "src", "dst"])
            .unwrap();
        let mut opts = ClientOpts::from_arg_matches(&matches).unwrap();
        opts.owner = true;
        opts.no_owner = false;
        opts.group = false;
        opts.no_group = true;
        #[cfg(unix)]
        {
            mock_effective_root(false);
            #[cfg(target_os = "linux")]
            mock_caps_has_cap(Ok(false));
        }
        check_privileges(&mut opts, &matches).unwrap();
        assert!(!opts.owner);
        assert!(opts.no_owner);
    }

    #[test]
    fn privilege_error_when_mapping() {
        let cmd = cli_command();
        let matches = cmd
            .clone()
            .try_get_matches_from(["prog", "src", "dst"])
            .unwrap();
        let mut opts = ClientOpts::from_arg_matches(&matches).unwrap();
        opts.chown = Some("0:0".into());
        #[cfg(unix)]
        {
            mock_effective_root(false);
            #[cfg(target_os = "linux")]
            mock_caps_has_cap(Ok(false));
        }
        let err = check_privileges(&mut opts, &matches).unwrap_err();
        matches!(err, EngineError::Exit(ExitCode::StartClient, _));
    }

    #[test]
    fn execute_transfer_local_ok() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        fs::write(src_dir.path().join("file"), b"data").unwrap();
        let src = RemoteSpec::Local(PathSpec {
            path: src_dir.path().to_path_buf(),
            trailing_slash: false,
        });
        let dst = RemoteSpec::Local(PathSpec {
            path: dst_dir.path().to_path_buf(),
            trailing_slash: false,
        });
        let matcher = Matcher::new(vec![]);
        let cmd = cli_command();
        let matches = cmd
            .clone()
            .try_get_matches_from(["prog", "src", "dst"])
            .unwrap();
        let opts = ClientOpts::from_arg_matches(&matches).unwrap();
        let rsh = RshCommand {
            env: vec![],
            cmd: vec![],
        };
        let rsync_env = vec![];
        let mut sync_opts = SyncOptions::default();
        let stats = execute_transfer(
            src,
            dst,
            &matcher,
            &opts,
            &rsh,
            &rsync_env,
            None,
            None,
            None,
            true,
            None,
            None,
            &mut sync_opts,
        )
        .unwrap();
        assert_eq!(stats.files_total, 1);
    }
}
