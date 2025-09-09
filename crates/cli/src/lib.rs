// crates/cli/src/lib.rs

#![allow(clippy::collapsible_if)]
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::ArgMatches;
#[cfg(test)]
use clap::FromArgMatches;

mod argparse;
pub mod branding;
pub mod daemon;
mod exec;
mod formatter;
mod print;
mod session;
mod transport_factory;
mod utils;
mod validate;

pub mod options {
    pub use super::argparse::*;
}

use crate::daemon::run_daemon;
use crate::options::{ClientOpts, ProbeOpts};
pub use daemon::spawn_daemon_session;
pub use utils::{
    PathSpec, RemoteSpec, parse_iconv, parse_logging_flags, parse_remote_spec, parse_rsh,
    print_version_if_requested,
};
use utils::{init_logging, parse_filters, parse_name_map, parse_remote_specs, parse_rsync_path};

use compress::{Codec, available_codecs};
pub use engine::EngineError;
use engine::{DeleteMode, Result, Stats, StrongHash, SyncOptions};
use filters::{Matcher, Rule, default_cvs_rules};
pub use formatter::{ARG_ORDER, dump_help_body, render_help};
use logging::{InfoFlag, parse_escapes};
use meta::{IdKind, parse_chmod, parse_chown};
use protocol::{SUPPORTED_PROTOCOLS, negotiate_version};
use transport::{AddressFamily, parse_sockopts};
#[cfg(unix)]
use users::get_user_by_uid;

pub mod version;

pub use options::cli_command;
pub use print::handle_clap_error;
pub use validate::{exit_code_from_engine_error, exit_code_from_error_kind};

pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    let (opts, probe_opts) = validate::parse_matches(matches)?;
    if opts.daemon.daemon {
        return run_daemon(opts.daemon, matches);
    }
    let log_file_fmt = opts.log_file_format.clone().map(|s| parse_escapes(&s));
    init_logging(matches, log_file_fmt)?;
    if matches.contains_id("probe") {
        return run_probe(probe_opts, matches.get_flag("quiet"));
    }
    run_client(opts, matches)
}

fn run_client(opts: ClientOpts, matches: &ArgMatches) -> Result<()> {
    let (srcs, dst_arg) = validate::validate_paths(&opts)?;
    let mut total = Stats::default();
    for src in srcs {
        let stats = run_single(opts.clone(), matches, src.as_os_str(), dst_arg.as_os_str())?;
        total.files_total += stats.files_total;
        total.dirs_total += stats.dirs_total;
        total.files_transferred += stats.files_transferred;
        total.files_deleted += stats.files_deleted;
        total.files_created += stats.files_created;
        total.dirs_created += stats.dirs_created;
        total.total_file_size += stats.total_file_size;
        total.bytes_transferred += stats.bytes_transferred;
        total.literal_data += stats.literal_data;
        total.matched_data += stats.matched_data;
        total.file_list_size += stats.file_list_size;
        total.file_list_gen_time += stats.file_list_gen_time;
        total.file_list_transfer_time += stats.file_list_transfer_time;
        total.bytes_sent += stats.bytes_sent;
        total.bytes_received += stats.bytes_received;
    }
    if opts.stats && !opts.quiet {
        print::print_stats(&total, &opts);
    }
    Ok(())
}

fn run_single(
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
    if opts.old_dirs {
        opts.dirs = true;
        opts.recursive = true;
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

    exec::check_privileges(&mut opts, matches)?;

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
    if opts.old_dirs {
        remote_opts.push("-r".into());
        remote_opts.push("--exclude=/*/*".into());
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

    let src_trailing = match &src {
        RemoteSpec::Local(p) => p.trailing_slash,
        RemoteSpec::Remote { path, .. } => path.trailing_slash,
    };
    let src_path = match &src {
        RemoteSpec::Local(p) => &p.path,
        RemoteSpec::Remote { path, .. } => &path.path,
    };
    let dst_is_dir = match &dst {
        RemoteSpec::Local(p) => p.path.is_dir(),
        RemoteSpec::Remote { .. } => true,
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
    } else if !src_trailing && dst_is_dir {
        let name = src_path
            .file_name()
            .map(|s| s.to_owned())
            .ok_or_else(|| EngineError::Other("source path missing file name".into()))?;
        match &mut dst {
            RemoteSpec::Local(p) => p.path.push(&name),
            RemoteSpec::Remote { path, .. } => path.path.push(&name),
        }
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
        dirs: opts.dirs,
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
    let stats = exec::execute_transfer(
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

fn build_matcher(opts: &ClientOpts, matches: &ArgMatches) -> Result<Matcher> {
    let mut entries: Vec<(usize, usize, Rule)> = Vec::new();
    let mut seq = 0;
    let mut add_rules = |idx: usize, rs: Vec<Rule>| {
        for r in rs {
            entries.push((idx, seq, r));
            seq += 1;
        }
    };

    if let Some(values) = matches.get_many::<String>("filter") {
        let idxs: Vec<_> = matches
            .indices_of("filter")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, val) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(val, opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("filter_file") {
        let idxs: Vec<_> = matches
            .indices_of("filter_file")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let rs = if file == Path::new("-") {
                if opts.from0 {
                    filters::parse_file(Path::new("-"), true, &mut HashSet::new(), 0)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?
                } else {
                    return Err(EngineError::Other(
                        "filter file '-' requires --from0".into(),
                    ));
                }
            } else {
                filters::parse_file(file, opts.from0, &mut HashSet::new(), 0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?
            };
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<String>("include") {
        let idxs: Vec<_> = matches
            .indices_of("include")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let mut bytes = pat.clone().into_bytes();
            if opts.from0 {
                bytes.push(0);
            } else {
                bytes.push(b'\n');
            }
            let rs =
                filters::parse_rule_list_from_bytes(&bytes, opts.from0, '+', &mut vset, 0, None)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<String>("exclude") {
        let idxs: Vec<_> = matches
            .indices_of("exclude")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(&format!("- {}", pat), opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("include_from") {
        let idxs: Vec<_> = matches
            .indices_of("include_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filters::parse_rule_list_file(file, opts.from0, '+', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("exclude_from") {
        let idxs: Vec<_> = matches
            .indices_of("exclude_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filters::parse_rule_list_file(file, opts.from0, '-', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("files_from") {
        let idxs: Vec<_> = matches
            .indices_of("files_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filters::parse_rule_list_file(file, opts.from0, '+', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if matches.contains_id("filter_shorthand") {
        if let Some(idx) = matches.index_of("filter_shorthand") {
            let count = matches.get_count("filter_shorthand");
            let rule_str = if count >= 2 { "-FF" } else { "-F" };
            add_rules(
                idx + 1,
                parse_filters(rule_str, opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if !opts.files_from.is_empty() {
        add_rules(
            usize::MAX,
            parse_filters("- /**", opts.from0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    if opts.cvs_exclude {
        let mut cvs = default_cvs_rules().map_err(|e| EngineError::Other(format!("{:?}", e)))?;
        cvs.extend(
            parse_filters(":C\n", opts.from0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
        add_rules(usize::MAX, cvs);
    }
    if opts.old_dirs {
        add_rules(
            usize::MAX,
            parse_filters("- /*/*", opts.from0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }

    entries.sort_by(|a, b| {
        if a.0 == b.0 {
            a.1.cmp(&b.1)
        } else {
            a.0.cmp(&b.0)
        }
    });
    let rules: Vec<Rule> = entries.into_iter().map(|(_, _, r)| r).collect();
    let mut matcher = Matcher::new(rules);
    if opts.from0 {
        matcher = matcher.with_from0();
    }
    if opts.existing {
        matcher = matcher.with_existing();
    }
    if opts.prune_empty_dirs {
        matcher = matcher.with_prune_empty_dirs();
    }
    if opts.no_implied_dirs {
        matcher = matcher.with_no_implied_dirs();
    }
    Ok(matcher)
}

fn run_probe(opts: ProbeOpts, quiet: bool) -> Result<()> {
    if let Some(addr) = opts.probe {
        let mut stream = TcpStream::connect(&addr)?;
        stream.write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(SUPPORTED_PROTOCOLS[0], peer)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    } else {
        let ver = negotiate_version(SUPPORTED_PROTOCOLS[0], opts.peer_version)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::{RemoteSpec, parse_bool, parse_remote_spec};
    use ::daemon::authenticate;
    use clap::Parser;
    use engine::SyncOptions;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[test]
    fn windows_paths_are_local() {
        let spec = parse_remote_spec(OsStr::new("C:/tmp/foo")).unwrap();
        assert!(matches!(spec, RemoteSpec::Local(_)));
    }

    #[test]
    fn parse_bool_is_case_insensitive() {
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("Yes").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("No").unwrap());
    }

    #[test]
    fn ipv6_specs_are_remote() {
        let spec = parse_remote_spec(OsStr::new("[::1]:/tmp")).unwrap();
        match spec {
            RemoteSpec::Remote { host, path, module } => {
                assert_eq!(host, "::1");
                assert!(module.is_none());
                assert_eq!(path.path, PathBuf::from("/tmp"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn no_d_alias_sets_no_devices_and_no_specials() {
        use crate::options::ClientOpts;
        let matches = cli_command()
            .try_get_matches_from(["prog", "--no-D", "src", "--", "dst"])
            .unwrap();
        let mut opts = ClientOpts::from_arg_matches(&matches).unwrap();
        if opts.no_D {
            opts.no_devices = true;
            opts.no_specials = true;
        }
        assert!(opts.no_devices);
        assert!(opts.no_specials);
    }

    #[test]
    fn run_client_errors_when_no_paths_provided() {
        use crate::options::ClientOpts;
        let mut opts = ClientOpts::try_parse_from(["prog", "--server"]).unwrap();
        opts.server = false;
        opts.paths.clear();
        let matches = cli_command()
            .try_get_matches_from(["prog", "--server"])
            .unwrap();
        let err = run_client(opts, &matches).unwrap_err();
        assert!(matches!(err, EngineError::Other(msg) if msg == "missing SRC or DST"));
    }

    #[test]
    fn rsync_url_specs_are_remote() {
        let spec = parse_remote_spec(OsStr::new("rsync://host/mod/path")).unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert_eq!(module.as_deref(), Some("mod"));
                assert_eq!(path.path, PathBuf::from("path"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn rsync_url_module_specs_are_remote() {
        let spec = parse_remote_spec(OsStr::new("rsync://host/mod")).unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert_eq!(module.as_deref(), Some("mod"));
                assert_eq!(path.path, PathBuf::from("."));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn daemon_double_colon_specs_are_remote() {
        let spec = parse_remote_spec(OsStr::new("host::mod/path")).unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert_eq!(module.as_deref(), Some("mod"));
                assert_eq!(path.path, PathBuf::from("path"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn host_path_specs_are_remote() {
        let spec = parse_remote_spec(OsStr::new("host:/tmp")).unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert!(module.is_none());
                assert_eq!(path.path, PathBuf::from("/tmp"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn malformed_rsync_url_is_error() {
        assert!(parse_remote_spec(OsStr::new("rsync://")).is_err());
    }

    #[test]
    fn malformed_daemon_spec_is_error() {
        assert!(parse_remote_spec(OsStr::new("host::mod")).is_err());
    }

    #[test]
    fn malformed_ipv6_spec_is_error() {
        assert!(parse_remote_spec(OsStr::new("[::1]:module")).is_err());
    }

    #[test]
    fn parses_client_flags() {
        let opts = ClientOpts::parse_from([
            "prog",
            "-r",
            "-n",
            "-v",
            "--delete",
            "-c",
            "-z",
            "--stats",
            "--executability",
            "--config",
            "file",
            "src",
            "dst",
        ]);
        assert!(opts.recursive);
        assert!(opts.dry_run);
        assert_eq!(opts.verbose, 1);
        assert!(opts.delete);
        assert!(opts.checksum);
        assert!(opts.compress);
        assert!(opts.stats);
        assert!(opts.executability);
        assert_eq!(opts.config, Some(PathBuf::from("file")));
    }

    #[test]
    fn parses_checksum_choice_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--checksum-choice", "sha1", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("sha1"));
        let opts = ClientOpts::parse_from(["prog", "--cc", "md5", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("md5"));
    }

    #[test]
    fn parses_rsh_flag_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--rsh", "ssh", "src", "dst"]);
        assert_eq!(opts.rsh.unwrap().cmd, vec!["ssh".to_string()]);
        let opts = ClientOpts::parse_from(["prog", "-e", "ssh", "src", "dst"]);
        assert_eq!(opts.rsh.unwrap().cmd, vec!["ssh".to_string()]);
    }

    #[test]
    fn parses_rsync_path_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--rsync-path", "/bin/rsync", "src", "dst"]);
        assert_eq!(opts.rsync_path.as_deref(), Some("/bin/rsync"));
        let opts = ClientOpts::parse_from(["prog", "--rsync_path", "/bin/rsync", "src", "dst"]);
        assert_eq!(opts.rsync_path.as_deref(), Some("/bin/rsync"));
    }

    #[test]
    fn parses_skip_compress_list() {
        let opts = ClientOpts::parse_from(["prog", "--skip-compress=GZ,Zip", "src", "dst"]);
        assert_eq!(opts.skip_compress, vec!["gz", "zip"]);
    }

    #[test]
    fn parses_skip_flags() {
        let opts = ClientOpts::parse_from([
            "prog",
            "--ignore-existing",
            "--existing",
            "--prune-empty-dirs",
            "--size-only",
            "--ignore-times",
            "src",
            "dst",
        ]);
        assert!(opts.ignore_existing);
        assert!(opts.existing);
        assert!(opts.prune_empty_dirs);
        assert!(opts.size_only);
        assert!(opts.ignore_times);
    }

    #[test]
    fn parses_protocol_version() {
        let opts = ClientOpts::parse_from(["prog", "--protocol", "30", "src", "dst"]);
        assert_eq!(opts.protocol, Some(30));
    }

    #[test]
    fn parses_8_bit_output() {
        let opts = ClientOpts::parse_from(["prog", "-8", "src", "dst"]);
        assert!(opts.eight_bit_output);
    }

    #[test]
    fn parses_blocking_io() {
        let opts = ClientOpts::parse_from(["prog", "--blocking-io", "src", "dst"]);
        assert!(opts.blocking_io);
    }

    #[test]
    fn parses_early_input() {
        let opts = ClientOpts::parse_from(["prog", "--early-input", "file", "src", "dst"]);
        assert_eq!(opts.early_input.as_deref(), Some(Path::new("file")));
    }

    #[test]
    fn protocol_override_sent_to_server() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(u32::from_be_bytes(buf), 30);
            stream
                .write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())
                .unwrap();

            let mut b = [0u8; 1];
            while stream.read(&mut b).unwrap() > 0 {
                if b[0] == b'\n' {
                    break;
                }
            }

            stream.write_all(b"@RSYNCD: OK\n").unwrap();

            let mut m = Vec::new();
            loop {
                stream.read_exact(&mut b).unwrap();
                if b[0] == b'\n' {
                    break;
                }
                m.push(b[0]);
            }
            assert_eq!(m, b"mod".to_vec());
        });

        let _t = spawn_daemon_session(
            "127.0.0.1",
            "mod",
            Some(port),
            None,
            true,
            None,
            None,
            None,
            &[],
            &SyncOptions::default(),
            30,
            None,
            None,
        )
        .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn sends_early_input_to_daemon() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path = dir.path().join("input.txt");
        fs::write(&path, b"hello\n").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 6];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"hello\n");
            let mut ver = [0u8; 4];
            stream.read_exact(&mut ver).unwrap();
            assert_eq!(u32::from_be_bytes(ver), 30);
            stream
                .write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())
                .unwrap();
            let mut b = [0u8; 1];
            while stream.read(&mut b).unwrap() > 0 {
                if b[0] == b'\n' {
                    break;
                }
            }
            stream.write_all(b"@RSYNCD: OK\n").unwrap();
            let mut m = Vec::new();
            loop {
                stream.read_exact(&mut b).unwrap();
                if b[0] == b'\n' {
                    break;
                }
                m.push(b[0]);
            }
            assert_eq!(m, b"mod".to_vec());
        });

        let _t = spawn_daemon_session(
            "127.0.0.1",
            "mod",
            Some(port),
            None,
            true,
            None,
            None,
            None,
            &[],
            &SyncOptions::default(),
            30,
            Some(&path),
            None,
        )
        .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn parses_internal_server_sender_flags() {
        let opts = ClientOpts::parse_from(["prog", "--server", "--sender", "src", "dst"]);
        assert!(opts.server);
        assert!(opts.sender);
    }

    #[test]
    fn rejects_invalid_env_assignment() {
        let err = parse_rsh(Some("1BAD=val ssh".into())).unwrap_err();
        assert!(matches!(err, EngineError::Other(_)));
    }

    #[test]
    #[cfg(unix)]
    fn rejects_insecure_auth_file() {
        use std::net::{TcpListener, TcpStream};
        use std::os::unix::fs::PermissionsExt;
        use std::{env, fs};
        use tempfile::tempdir;
        use transport::TcpTransport;

        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth");
        fs::write(&auth_path, "tok user").unwrap();
        fs::set_permissions(&auth_path, fs::Permissions::from_mode(0o644)).unwrap();

        let prev = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (_s, _) = listener.accept().unwrap();
        });
        let stream = TcpStream::connect(addr).unwrap();
        let mut t = TcpTransport::from_stream(stream);

        let err = authenticate(&mut t, None, None).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);

        env::set_current_dir(prev).unwrap();
        handle.join().unwrap();
    }
}
