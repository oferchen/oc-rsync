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
    Misc,
    Name,
    Progress,
    Stats,
}

impl InfoFlag {
    pub const fn as_str(self) -> &'static str {
        match self {
            InfoFlag::Backup => "backup",
            InfoFlag::Copy => "copy",
            InfoFlag::Del => "del",
            InfoFlag::Flist => "flist",
            InfoFlag::Misc => "misc",
            InfoFlag::Name => "name",
            InfoFlag::Progress => "progress",
            InfoFlag::Stats => "stats",
        }
    }

    pub const fn target(self) -> &'static str {
        match self {
            InfoFlag::Backup => "info::backup",
            InfoFlag::Copy => "info::copy",
            InfoFlag::Del => "info::del",
            InfoFlag::Flist => "info::flist",
            InfoFlag::Misc => "info::misc",
            InfoFlag::Name => "info::name",
            InfoFlag::Progress => "info::progress",
            InfoFlag::Stats => "info::stats",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum DebugFlag {
    Backup,
    Copy,
    Del,
    Flist,
    Hash,
    Match,
    Misc,
    Options,
}

impl DebugFlag {
    pub const fn as_str(self) -> &'static str {
        match self {
            DebugFlag::Backup => "backup",
            DebugFlag::Copy => "copy",
            DebugFlag::Del => "del",
            DebugFlag::Flist => "flist",
            DebugFlag::Hash => "hash",
            DebugFlag::Match => "match",
            DebugFlag::Misc => "misc",
            DebugFlag::Options => "options",
        }
    }

    pub const fn target(self) -> &'static str {
        match self {
            DebugFlag::Backup => "debug::backup",
            DebugFlag::Copy => "debug::copy",
            DebugFlag::Del => "debug::del",
            DebugFlag::Flist => "debug::flist",
            DebugFlag::Hash => "debug::hash",
            DebugFlag::Match => "debug::match",
            DebugFlag::Misc => "debug::misc",
            DebugFlag::Options => "debug::options",
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
    } else if !debug.is_empty() || verbose > 1 {
        LevelFilter::DEBUG
    } else if !info.is_empty() || verbose > 0 {
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
                format!("{}=debug", flag.target()).parse().unwrap();
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
