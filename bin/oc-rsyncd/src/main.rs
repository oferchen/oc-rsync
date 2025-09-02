// bin/oc-rsyncd/src/main.rs
use daemon::{load_config, parse_daemon_args, run_daemon, Handler, Module};
use oc_rsync_cli::version;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use transport::AddressFamily;

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        if !std::env::args().any(|a| a == "--quiet" || a == "-q") {
            println!("{}", version::render_version_lines().join("\n"));
        }
        return;
    }

    let mut config: Option<PathBuf> = None;
    let mut args = Vec::new();
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--config" {
            if let Some(p) = iter.next() {
                config = Some(PathBuf::from(p));
            }
        } else if let Some(rest) = arg.strip_prefix("--config=") {
            config = Some(PathBuf::from(rest));
        } else {
            args.push(arg);
        }
    }

    let opts = parse_daemon_args(args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let cfg = load_config(config.as_deref()).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let mut modules: HashMap<String, Module> = cfg
        .modules
        .into_iter()
        .map(|m| (m.name.clone(), m))
        .collect();

    if let Some(val) = cfg.use_chroot {
        for m in modules.values_mut() {
            m.use_chroot = val;
        }
    }
    if let Some(val) = cfg.numeric_ids {
        for m in modules.values_mut() {
            m.numeric_ids = val;
        }
    }
    if let Some(val) = cfg.read_only {
        for m in modules.values_mut() {
            m.read_only = val;
        }
    }
    if let Some(val) = cfg.write_only {
        for m in modules.values_mut() {
            m.write_only = val;
        }
    }
    if !cfg.refuse_options.is_empty() {
        for m in modules.values_mut() {
            m.refuse_options = cfg.refuse_options.clone();
        }
    }

    let list = cfg.list.unwrap_or(true);
    let max_conn = cfg.max_connections;

    let mut port = opts.port;
    if let Some(p) = cfg.port {
        port = p;
    }
    let mut address = opts.address;
    let mut family = opts.family;
    if let Some(a) = cfg.address6 {
        address = Some(a);
        family = Some(AddressFamily::V6);
    } else if let Some(a) = cfg.address {
        address = Some(a);
        family = Some(AddressFamily::V4);
    }

    let uid = cfg.uid.unwrap_or(65534);
    let gid = cfg.gid.unwrap_or(65534);

    let handler: Arc<Handler> = Arc::new(|_| Ok(()));

    if let Err(e) = run_daemon(
        modules,
        cfg.secrets_file,
        None,
        cfg.hosts_allow,
        cfg.hosts_deny,
        cfg.log_file,
        None,
        cfg.motd_file,
        cfg.pid_file,
        cfg.lock_file,
        None,
        cfg.timeout,
        None,
        max_conn,
        cfg.refuse_options,
        list,
        port,
        address,
        family,
        uid,
        gid,
        handler,
        false,
    ) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
