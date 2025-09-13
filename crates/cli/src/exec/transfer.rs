// crates/cli/src/exec/transfer.rs

use std::path::{Path, PathBuf};

use crate::options::ClientOpts;
use crate::utils::{RemoteSpec, RshCommand};
use crate::EngineError;

use oc_rsync_core::{
    compress::available_codecs,
    config::SyncOptions,
    filter::Matcher,
    message::{CharsetConv, CAP_ACLS, CAP_CODECS, CAP_XATTRS},
    transfer::{sync, Result, Stats},
};
use transport::{AddressFamily, SshStdioTransport};

mod remote_remote;

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
    let stats = match (src, dst) {
        (RemoteSpec::Local(src), RemoteSpec::Local(dst)) => sync(
            &src.path,
            &dst.path,
            matcher,
            &available_codecs(),
            sync_opts,
        )?,
        (
            RemoteSpec::Remote {
                host,
                path: src,
                module: Some(module),
            },
            RemoteSpec::Local(dst),
        ) => {
            let remote_src =
                PathBuf::from(format!("rsync://{host}/{module}/{}", src.path.display()));
            sync(
                &remote_src,
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
            let remote_dst =
                PathBuf::from(format!("rsync://{host}/{module}/{}", dst.path.display()));
            sync(
                &src.path,
                &remote_dst,
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
        ) => remote_remote::remote_to_remote(
            src_host,
            src_path,
            src_module,
            dst_host,
            dst_path,
            dst_module,
            opts,
            rsh_cmd,
            remote_bin,
            remote_env,
            known_hosts,
            strict_host_key_checking,
            addr_family,
            iconv,
            sync_opts,
        )?,
    };
    Ok(stats)
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
