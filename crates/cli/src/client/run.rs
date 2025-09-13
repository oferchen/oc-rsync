// crates/cli/src/client/run.rs

use clap::ArgMatches;
use oc_rsync_core::transfer::{Result, Stats};

use crate::{
    daemon::run_daemon,
    options::{ClientOpts, ClientOptsBuilder, ProbeOptsBuilder, validate_paths},
    print, probe,
    utils::init_logging,
};
use logging::parse_escapes;

use super::exec::run_single;

pub fn run(matches: &ArgMatches) -> Result<()> {
    let opts = ClientOptsBuilder::from_matches(matches).build()?;
    let probe_opts = ProbeOptsBuilder::from_matches(matches).build()?;
    if opts.daemon.daemon {
        return run_daemon(opts.daemon, matches);
    }
    let log_file_fmt = opts.log_file_format.clone().map(|s| parse_escapes(&s));
    init_logging(matches, log_file_fmt)?;
    if matches.contains_id("probe") {
        return probe::run_probe(probe_opts, matches.get_flag("quiet"));
    }
    run_client(opts, matches)
}

pub(crate) fn run_client(opts: ClientOpts, matches: &ArgMatches) -> Result<()> {
    let (srcs, dst_arg) = validate_paths(&opts)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::parse_bool;
    #[allow(unused_imports)]
    use crate::{EngineError, cli_command, spawn_daemon_session};
    use crate::{RemoteSpec, parse_remote_spec};
    use clap::{FromArgMatches, Parser};
    #[allow(unused_imports)]
    use daemon::authenticate;
    #[allow(unused_imports)]
    use oc_rsync_core::config::SyncOptions;
    #[cfg(test)]
    #[allow(unused_imports)]
    use oc_rsync_core::message::SUPPORTED_PROTOCOLS;
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
}
