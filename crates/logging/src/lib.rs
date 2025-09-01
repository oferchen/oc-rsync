// crates/logging/src/lib.rs
use std::fs::OpenOptions;
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt,
    layer::{Layer as _, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter,
};

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum InfoFlag {
    Backup,
    Copy,
    Del,
    Flist,
    Flist2,
    Misc,
    Misc2,
    Mount,
    Name,
    Name2,
    Nonreg,
    Progress,
    Progress2,
    Remove,
    Skip,
    Skip2,
    Stats,
    Stats2,
    Stats3,
    Symsafe,
}

impl InfoFlag {
    pub const fn as_str(self) -> &'static str {
        match self {
            InfoFlag::Backup => "backup",
            InfoFlag::Copy => "copy",
            InfoFlag::Del => "del",
            InfoFlag::Flist => "flist",
            InfoFlag::Flist2 => "flist2",
            InfoFlag::Misc => "misc",
            InfoFlag::Misc2 => "misc2",
            InfoFlag::Mount => "mount",
            InfoFlag::Name => "name",
            InfoFlag::Name2 => "name2",
            InfoFlag::Nonreg => "nonreg",
            InfoFlag::Progress => "progress",
            InfoFlag::Progress2 => "progress2",
            InfoFlag::Remove => "remove",
            InfoFlag::Skip => "skip",
            InfoFlag::Skip2 => "skip2",
            InfoFlag::Stats => "stats",
            InfoFlag::Stats2 => "stats2",
            InfoFlag::Stats3 => "stats3",
            InfoFlag::Symsafe => "symsafe",
        }
    }

    pub const fn target(self) -> &'static str {
        match self {
            InfoFlag::Backup => "info::backup",
            InfoFlag::Copy => "info::copy",
            InfoFlag::Del => "info::del",
            InfoFlag::Flist | InfoFlag::Flist2 => "info::flist",
            InfoFlag::Misc | InfoFlag::Misc2 => "info::misc",
            InfoFlag::Mount => "info::mount",
            InfoFlag::Name | InfoFlag::Name2 => "info::name",
            InfoFlag::Nonreg => "info::nonreg",
            InfoFlag::Progress | InfoFlag::Progress2 => "info::progress",
            InfoFlag::Remove => "info::remove",
            InfoFlag::Skip | InfoFlag::Skip2 => "info::skip",
            InfoFlag::Stats | InfoFlag::Stats2 | InfoFlag::Stats3 => "info::stats",
            InfoFlag::Symsafe => "info::symsafe",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum DebugFlag {
    Acl,
    Backup,
    Bind,
    Chdir,
    Connect,
    Cmd,
    Del,
    Deltasum,
    Dup,
    Exit,
    Filter,
    Flist,
    Fuzzy,
    Genr,
    Hash,
    Hlink,
    Iconv,
    Io,
    Nstr,
    Own,
    Proto,
    Recv,
    Send,
    Time,
}

impl DebugFlag {
    pub const fn as_str(self) -> &'static str {
        match self {
            DebugFlag::Acl => "acl",
            DebugFlag::Backup => "backup",
            DebugFlag::Bind => "bind",
            DebugFlag::Chdir => "chdir",
            DebugFlag::Connect => "connect",
            DebugFlag::Cmd => "cmd",
            DebugFlag::Del => "del",
            DebugFlag::Deltasum => "deltasum",
            DebugFlag::Dup => "dup",
            DebugFlag::Exit => "exit",
            DebugFlag::Filter => "filter",
            DebugFlag::Flist => "flist",
            DebugFlag::Fuzzy => "fuzzy",
            DebugFlag::Genr => "genr",
            DebugFlag::Hash => "hash",
            DebugFlag::Hlink => "hlink",
            DebugFlag::Iconv => "iconv",
            DebugFlag::Io => "io",
            DebugFlag::Nstr => "nstr",
            DebugFlag::Own => "own",
            DebugFlag::Proto => "proto",
            DebugFlag::Recv => "recv",
            DebugFlag::Send => "send",
            DebugFlag::Time => "time",
        }
    }

    pub const fn target(self) -> &'static str {
        match self {
            DebugFlag::Acl => "debug::acl",
            DebugFlag::Backup => "debug::backup",
            DebugFlag::Bind => "debug::bind",
            DebugFlag::Chdir => "debug::chdir",
            DebugFlag::Connect => "debug::connect",
            DebugFlag::Cmd => "debug::cmd",
            DebugFlag::Del => "debug::del",
            DebugFlag::Deltasum => "debug::deltasum",
            DebugFlag::Dup => "debug::dup",
            DebugFlag::Exit => "debug::exit",
            DebugFlag::Filter => "debug::filter",
            DebugFlag::Flist => "debug::flist",
            DebugFlag::Fuzzy => "debug::fuzzy",
            DebugFlag::Genr => "debug::genr",
            DebugFlag::Hash => "debug::hash",
            DebugFlag::Hlink => "debug::hlink",
            DebugFlag::Iconv => "debug::iconv",
            DebugFlag::Io => "debug::io",
            DebugFlag::Nstr => "debug::nstr",
            DebugFlag::Own => "debug::own",
            DebugFlag::Proto => "debug::proto",
            DebugFlag::Recv => "debug::recv",
            DebugFlag::Send => "debug::send",
            DebugFlag::Time => "debug::time",
        }
    }
}

pub fn subscriber(
    format: LogFormat,
    verbose: u8,
    info: &[InfoFlag],
    debug: &[DebugFlag],
    quiet: bool,
    log_file: Option<(PathBuf, Option<String>)>,
) -> Box<dyn tracing::Subscriber + Send + Sync> {
    let level = if quiet {
        LevelFilter::ERROR
    } else if verbose > 2 {
        LevelFilter::TRACE
    } else if verbose > 1 {
        LevelFilter::DEBUG
    } else if verbose > 0 {
        LevelFilter::INFO
    } else {
        LevelFilter::WARN
    };
    let mut filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    if !quiet {
        for flag in info {
            let directive: tracing_subscriber::filter::Directive =
                format!("{}=info", flag.target()).parse().unwrap();
            filter = filter.add_directive(directive);
        }
        for flag in debug {
            let directive: tracing_subscriber::filter::Directive =
                format!("{}=trace", flag.target()).parse().unwrap();
            filter = filter.add_directive(directive);
        }
    }

    let fmt_layer = fmt::layer();
    let fmt_layer = match format {
        LogFormat::Json => fmt_layer.json().boxed(),
        LogFormat::Text => fmt_layer.boxed(),
    };
    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);
    if let Some((path, fmt)) = log_file {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("failed to open log file");
        let base = fmt::layer()
            .with_writer(move || file.try_clone().unwrap())
            .with_ansi(false);
        let file_layer = match fmt.as_deref() {
            Some("json") => base.json().boxed(),
            _ => base.boxed(),
        };
        Box::new(registry.with(file_layer))
    } else {
        Box::new(registry)
    }
}

pub fn init(
    format: LogFormat,
    verbose: u8,
    info: &[InfoFlag],
    debug: &[DebugFlag],
    quiet: bool,
    log_file: Option<(PathBuf, Option<String>)>,
) {
    subscriber(format, verbose, info, debug, quiet, log_file).init();
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{}{}", bytes, UNITS[unit])
    } else {
        format!("{:.2}{}", size, UNITS[unit])
    }
}

pub fn progress_formatter(bytes: u64, human_readable: bool) -> String {
    if human_readable {
        human_bytes(bytes)
    } else {
        let s = bytes.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out.chars().rev().collect()
    }
}
