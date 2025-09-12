// crates/cli/src/client/exec.rs

use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;

use clap::ArgMatches;
use logging::{InfoFlag, parse_escapes};
use oc_rsync_core::{
    compress::{Codec, available_codecs},
    config::{DeleteMode, SyncOptions},
    fs::{IdKind, parse_chmod, parse_chown},
    transfer::{Result, Stats, StrongHash},
};
use transport::{AddressFamily, parse_sockopts};
#[cfg(unix)]
use users::get_user_by_uid;

use crate::exec::{check_privileges, execute_transfer};
use crate::{
    EngineError, RemoteSpec,
    options::ClientOpts,
    utils::{parse_iconv, parse_name_map, parse_remote_specs, parse_rsh, parse_rsync_path},
};

use super::args::build_matcher;

pub(crate) fn run_single(
    mut opts: ClientOpts,
    matches: &ArgMatches,
    src_arg: &OsStr,
    dst_arg: &OsStr,
) -> Result<Stats> {
    if opts.archive {
        opts.recursive = true;
        opts.links = true;
        opts.perms = !opts.no_perms;
        opts.times = !opts.no_times;
        opts.group = !opts.no_group;
        opts.owner = !opts.no_owner;
        opts.devices = !opts.no_devices;
        opts.specials = !opts.no_specials;
    }
    if opts.no_links {
        opts.links = false;
    }

    if !opts.files_from.is_empty() {
        opts.dirs = true;
        opts.relative = true;
    }
    let matcher = build_matcher(&opts, matches)?;
    let addr_family = if opts.ipv4 {
        Some(AddressFamily::V4)
    } else if opts.ipv6 {
        Some(AddressFamily::V6)
    } else {
        None
    };

    parse_sockopts(&opts.sockopts).map_err(EngineError::Other)?;

    let acls = opts.acls && !opts.no_acls;

    check_privileges(&mut opts, matches)?;

    let iconv = if let Some(spec) = &opts.iconv {
        Some(parse_iconv(spec).map_err(EngineError::Other)?)
    } else {
        None
    };

    if let Some(pf) = &opts.password_file {
        #[cfg(unix)]
        {
            let mode = fs::metadata(pf)?.permissions().mode();
            if mode & 0o077 != 0 {
                return Err(EngineError::Other(
                    "password file permissions are too open".into(),
                ));
            }
        }
        let _ = fs::read_to_string(pf).map_err(|e| EngineError::Other(e.to_string()))?;
    }

    let mut remote_opts = opts.remote_option.clone();
    if opts.secluded_args {
        remote_opts.push("--secluded-args".into());
    }
    if opts.trust_sender {
        remote_opts.push("--trust-sender".into());
    }
    if let Some(spec) = &opts.iconv {
        remote_opts.push(format!("--iconv={spec}"));
    }
    if opts.xattrs {
        remote_opts.push("--xattrs".into());
    }
    if acls {
        remote_opts.push("--acls".into());
    }
    if opts.old_args {
        remote_opts.push("--old-args".into());
    }

    if let Some(cfg) = &opts.config {
        if !opts.quiet {
            println!("using config file {}", cfg.display());
        }
    }
    if opts.verbose > 0 && !opts.quiet {
        tracing::info!(
            target: InfoFlag::Misc.target(),
            "verbose level set to {}",
            opts.verbose
        );
    }
    if opts.verbose > 0 && opts.recursive && !opts.quiet {
        tracing::info!(target: InfoFlag::Misc.target(), "recursive mode enabled");
    }
    let (src, mut dst) = parse_remote_specs(src_arg, dst_arg)?;
    if opts.mkpath {
        match &dst {
            RemoteSpec::Local(ps) => {
                let target = if ps.trailing_slash {
                    &ps.path
                } else {
                    ps.path.parent().unwrap_or(&ps.path)
                };
                fs::create_dir_all(target).map_err(|e| EngineError::Other(e.to_string()))?;
            }
            RemoteSpec::Remote { .. } => {
                remote_opts.push("--mkpath".into());
            }
        }
    }

    let known_hosts = opts.known_hosts.clone();
    let strict_host_key_checking = !opts.no_host_key_checking;
    let rsh_cmd = match opts.rsh.clone() {
        Some(cmd) => cmd,
        None => parse_rsh(env::var("RSYNC_RSH").ok().or_else(|| env::var("RSH").ok()))?,
    };
    let rsync_path_cmd = parse_rsync_path(opts.rsync_path.clone())?;
    let mut rsync_env: Vec<(String, String)> = env::vars()
        .filter(|(k, _)| k.starts_with("RSYNC_"))
        .collect();
    rsync_env.extend(
        rsh_cmd
            .env
            .iter()
            .filter(|(k, _)| k.starts_with("RSYNC_"))
            .cloned(),
    );
    if let Some(cmd) = &rsync_path_cmd {
        rsync_env.extend(
            cmd.env
                .iter()
                .filter(|(k, _)| k.starts_with("RSYNC_"))
                .cloned(),
        );
    }
    if let Some(to) = opts.timeout {
        rsync_env.push(("RSYNC_TIMEOUT".into(), to.as_secs().to_string()));
    }

    if !rsync_env.iter().any(|(k, _)| k == "RSYNC_CHECKSUM_LIST") {
        let list: Vec<&str> = vec!["md4", "md5", "sha1"];
        rsync_env.push(("RSYNC_CHECKSUM_LIST".into(), list.join(",")));
    }

    let remote_bin_vec = rsync_path_cmd.as_ref().map(|c| c.cmd.clone());
    let remote_env_vec = rsync_path_cmd.as_ref().map(|c| c.env.clone());

    let strong = if let Some(choice) = opts.checksum_choice.as_deref() {
        match choice {
            "md4" => StrongHash::Md4,
            "md5" => StrongHash::Md5,
            "sha1" => StrongHash::Sha1,
            other => {
                return Err(EngineError::Other(format!("unknown checksum {other}")));
            }
        }
    } else if let Ok(list) = env::var("RSYNC_CHECKSUM_LIST") {
        let mut chosen = StrongHash::Md4;
        for name in list.split(',') {
            match name {
                "sha1" => {
                    chosen = StrongHash::Sha1;
                    break;
                }
                "md5" => {
                    chosen = StrongHash::Md5;
                    break;
                }
                "md4" => {
                    chosen = StrongHash::Md4;
                    break;
                }
                _ => {}
            }
        }
        chosen
    } else {
        StrongHash::Md4
    };

    let src_path = match &src {
        RemoteSpec::Local(p) => &p.path,
        RemoteSpec::Remote { path, .. } => &path.path,
    };
    if opts.relative {
        let rel = if src_path.is_absolute() {
            src_path.strip_prefix(Path::new("/")).unwrap_or(src_path)
        } else {
            src_path
        };
        match &mut dst {
            RemoteSpec::Local(p) => p.path.push(rel),
            RemoteSpec::Remote { path, .. } => path.path.push(rel),
        }
    }
    match &mut dst {
        RemoteSpec::Local(p) if !p.path.is_dir() => {
            if let Some(parent) = p.path.parent() {
                p.path = parent.to_path_buf();
            }
        }
        _ => {}
    }

    let compress_choice = match opts.compress_choice.as_deref() {
        Some("none") => None,
        Some(s) => {
            let mut list = Vec::new();
            for name in s.split(',') {
                let codec = match name {
                    "zlib" => Codec::Zlib,
                    "zlibx" => Codec::ZlibX,
                    "zstd" => Codec::Zstd,
                    other => {
                        return Err(EngineError::Other(format!("unknown codec {other}")));
                    }
                };
                if !available_codecs().contains(&codec) {
                    return Err(EngineError::Other(format!(
                        "codec {name} not supported by this build"
                    )));
                }
                list.push(codec);
            }
            if list.is_empty() { None } else { Some(list) }
        }
        None => None,
    };
    let compress = if opts.compress_choice.as_deref() == Some("none") {
        false
    } else {
        opts.compress || opts.compress_level.is_some_and(|l| l > 0) || compress_choice.is_some()
    };
    let mut delete_mode = if opts.delete_before {
        Some(DeleteMode::Before)
    } else if opts.delete_after || opts.delete_delay {
        Some(DeleteMode::After)
    } else if opts.delete_during || opts.delete {
        Some(DeleteMode::During)
    } else {
        None
    };
    if delete_mode.is_none() && opts.delete_excluded {
        delete_mode = Some(DeleteMode::During);
    }
    let block_size = opts.block_size.unwrap_or(0);
    let mut chmod_rules = Vec::new();
    for spec in &opts.chmod {
        chmod_rules.extend(parse_chmod(spec).map_err(EngineError::Other)?);
    }
    let chown_ids = if let Some(ref spec) = opts.chown {
        Some(parse_chown(spec).map_err(EngineError::Other)?)
    } else {
        None
    };
    let copy_as = if let Some(ref spec) = opts.copy_as {
        let (uid_opt, gid_opt) = parse_chown(spec).map_err(EngineError::Other)?;
        let uid = uid_opt.ok_or_else(|| EngineError::Other("--copy-as requires a user".into()))?;
        let gid = if let Some(g) = gid_opt {
            Some(g)
        } else {
            #[cfg(unix)]
            {
                get_user_by_uid(uid).map(|u| u.primary_group_id())
            }
            #[cfg(not(unix))]
            {
                None
            }
        };
        Some((uid, gid))
    } else {
        None
    };
    let uid_map = parse_name_map(&opts.usermap, IdKind::User)?;
    let gid_map = parse_name_map(&opts.groupmap, IdKind::Group)?;
    let (write_batch, only_write_batch) =
        match (opts.write_batch.clone(), opts.only_write_batch.clone()) {
            (Some(p), None) => (Some(p), false),
            (None, Some(p)) => (Some(p), true),
            (None, None) => (None, false),
            _ => unreachable!(),
        };
    let mut sync_opts = SyncOptions {
        delete: delete_mode,
        delete_excluded: opts.delete_excluded,
        ignore_missing_args: opts.ignore_missing_args,
        delete_missing_args: opts.delete_missing_args,
        remove_source_files: opts.remove_source_files,
        ignore_errors: opts.ignore_errors,
        force: opts.force,
        max_delete: opts.max_delete,
        max_alloc: opts.max_alloc.unwrap_or(1usize << 30),
        max_size: opts.max_size,
        min_size: opts.min_size,
        preallocate: opts.preallocate,
        checksum: opts.checksum,
        compress,
        dirs_only: opts.dirs,
        no_implied_dirs: opts.no_implied_dirs,
        dry_run: opts.dry_run,
        list_only: opts.list_only,
        update: opts.update,
        existing: opts.existing,
        ignore_existing: opts.ignore_existing,
        one_file_system: opts.one_file_system,
        size_only: opts.size_only,
        ignore_times: opts.ignore_times,
        perms: if opts.no_perms {
            false
        } else {
            opts.perms || opts.archive || acls
        },
        executability: opts.executability,
        times: if opts.no_times {
            false
        } else {
            opts.times || opts.archive
        },
        atimes: opts.atimes,
        crtimes: opts.crtimes,
        omit_dir_times: opts.omit_dir_times,
        omit_link_times: opts.omit_link_times,
        owner: if opts.no_owner {
            false
        } else {
            opts.owner
                || opts.archive
                || chown_ids.is_some_and(|(u, _)| u.is_some())
                || uid_map.is_some()
        },
        group: if opts.no_group {
            false
        } else {
            opts.group
                || opts.archive
                || chown_ids.is_some_and(|(_, g)| g.is_some())
                || gid_map.is_some()
        },
        links: opts.links,
        copy_links: opts.copy_links,
        copy_dirlinks: opts.copy_dirlinks,
        keep_dirlinks: opts.keep_dirlinks,
        copy_unsafe_links: opts.copy_unsafe_links,
        safe_links: opts.safe_links,
        munge_links: opts.munge_links,
        hard_links: opts.hard_links,
        devices: if opts.no_devices {
            false
        } else {
            opts.devices || opts.archive || opts.devices_specials
        },
        specials: if opts.no_specials {
            false
        } else {
            opts.specials || opts.archive || opts.devices_specials
        },
        xattrs: opts.xattrs || (opts.fake_super && !opts.super_user),
        acls,
        sparse: opts.sparse,
        strong,
        checksum_seed: opts.checksum_seed.unwrap_or_default(),
        compress_level: opts.compress_level,
        compress_choice,
        whole_file: if opts.no_whole_file {
            false
        } else {
            opts.whole_file
        },
        skip_compress: opts.skip_compress.iter().cloned().collect::<HashSet<_>>(),
        partial: opts.partial
            || opts.partial_progress
            || opts.partial_dir.is_some()
            || opts.append
            || opts.append_verify,
        progress: opts.progress || opts.partial_progress,
        human_readable: opts.human_readable,
        itemize_changes: opts.itemize_changes,
        out_format: opts.out_format.as_ref().map(|s| parse_escapes(s)),
        partial_dir: opts.partial_dir.clone(),
        temp_dir: opts.temp_dir.clone(),
        append: opts.append,
        append_verify: opts.append_verify,
        numeric_ids: opts.numeric_ids,
        inplace: opts.inplace || opts.write_devices,
        delay_updates: opts.delay_updates,
        modify_window: opts.modify_window.unwrap_or(Duration::ZERO),
        bwlimit: opts.bwlimit,
        stop_after: opts.stop_after,
        stop_at: opts.stop_at,
        block_size,
        link_dest: opts.link_dest.clone(),
        copy_dest: opts.copy_dest.clone(),
        compare_dest: opts.compare_dest.clone(),
        backup: opts.backup || opts.backup_dir.is_some(),
        backup_dir: opts.backup_dir.clone(),
        backup_suffix: opts.suffix.clone().unwrap_or_else(|| {
            if opts.backup_dir.is_some() {
                String::new()
            } else {
                "~".into()
            }
        }),
        chmod: if chmod_rules.is_empty() {
            None
        } else {
            Some(chmod_rules)
        },
        chown: chown_ids,
        copy_as,
        uid_map,
        gid_map,
        eight_bit_output: opts.eight_bit_output,
        blocking_io: opts.blocking_io,
        open_noatime: opts.open_noatime,
        early_input: opts.early_input.clone(),
        secluded_args: opts.secluded_args,
        sockopts: opts.sockopts.clone(),
        remote_options: remote_opts.clone(),
        write_batch,
        only_write_batch,
        read_batch: opts.read_batch.clone(),
        copy_devices: opts.copy_devices,
        write_devices: opts.write_devices,
        fsync: opts.fsync,
        fuzzy: opts.fuzzy,
        super_user: opts.super_user,
        fake_super: opts.fake_super && !opts.super_user,
        quiet: opts.quiet,
    };
    let stats = execute_transfer(
        src,
        dst,
        &matcher,
        &opts,
        &rsh_cmd,
        &rsync_env,
        remote_bin_vec.as_deref(),
        remote_env_vec.as_deref(),
        known_hosts.as_deref(),
        strict_host_key_checking,
        addr_family,
        iconv.as_ref(),
        &mut sync_opts,
    )?;
    Ok(stats)
}
