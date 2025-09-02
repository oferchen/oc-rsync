// crates/logging/src/lib.rs
use std::fmt;
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt as tracing_fmt,
    layer::{Context, Layer, SubscriberExt},
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

#[cfg(unix)]
struct MessageVisitor {
    msg: String,
}

#[cfg(unix)]
impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if !self.msg.is_empty() {
            self.msg.push(' ');
        }
        self.msg.push_str(field.name());
        self.msg.push('=');
        self.msg.push_str(value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if !self.msg.is_empty() {
            self.msg.push(' ');
        }
        self.msg.push_str(field.name());
        self.msg.push('=');
        self.msg.push_str(&format!("{value:?}"));
    }
}

#[cfg(unix)]
struct SyslogLayer {
    sock: UnixDatagram,
}

#[cfg(unix)]
impl SyslogLayer {
    fn new() -> std::io::Result<Self> {
        let path = std::env::var_os("OC_RSYNC_SYSLOG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/dev/log"));
        let sock = UnixDatagram::unbound()?;
        sock.connect(path)?;
        Ok(Self { sock })
    }
}

#[cfg(unix)]
fn syslog_severity(level: Level) -> u8 {
    match level {
        Level::ERROR => 3,
        Level::WARN => 4,
        Level::INFO => 6,
        Level::DEBUG | Level::TRACE => 7,
    }
}

#[cfg(unix)]
impl<S> Layer<S> for SyslogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event, _ctx: Context<S>) {
        let mut v = MessageVisitor { msg: String::new() };
        event.record(&mut v);
        if v.msg.is_empty() {
            v.msg.push_str(event.metadata().target());
        }
        let pri = 8 + syslog_severity(*event.metadata().level());
        let data = format!("<{pri}>{}", v.msg);
        let _ = self.sock.send(data.as_bytes());
    }
}

#[cfg(unix)]
struct JournaldLayer {
    sock: UnixDatagram,
}

#[cfg(unix)]
impl JournaldLayer {
    fn new() -> std::io::Result<Self> {
        let path = std::env::var_os("OC_RSYNC_JOURNALD_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/run/systemd/journal/socket"));
        let sock = UnixDatagram::unbound()?;
        sock.connect(path)?;
        Ok(Self { sock })
    }
}

#[cfg(unix)]
fn journald_priority(level: Level) -> u8 {
    match level {
        Level::ERROR => 3,
        Level::WARN => 4,
        Level::INFO => 6,
        Level::DEBUG | Level::TRACE => 7,
    }
}

#[cfg(unix)]
impl<S> Layer<S> for JournaldLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event, _ctx: Context<S>) {
        let mut v = MessageVisitor { msg: String::new() };
        event.record(&mut v);
        if v.msg.is_empty() {
            v.msg.push_str(event.metadata().target());
        }
        let prio = journald_priority(*event.metadata().level());
        let data = format!("PRIORITY={prio}\nMESSAGE={}\n", v.msg);
        let _ = self.sock.send(data.as_bytes());
    }
}

pub fn subscriber(
    format: LogFormat,
    verbose: u8,
    info: &[InfoFlag],
    debug: &[DebugFlag],
    quiet: bool,
    log_file: Option<(PathBuf, Option<String>)>,
    syslog: bool,
    journald: bool,
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

    let fmt_layer = tracing_fmt::layer();
    let fmt_layer = match format {
        LogFormat::Json => fmt_layer.json().boxed(),
        LogFormat::Text => fmt_layer.boxed(),
    };

    #[cfg(unix)]
    let syslog_layer = if syslog {
        SyslogLayer::new().ok()
    } else {
        None
    };
    #[cfg(unix)]
    let journald_layer = if journald {
        JournaldLayer::new().ok()
    } else {
        None
    };

    #[cfg(unix)]
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(syslog_layer)
        .with(journald_layer);

    #[cfg(not(unix))]
    let registry = {
        let _ = (syslog, journald);
        tracing_subscriber::registry().with(filter).with(fmt_layer)
    };

    let file_layer = log_file.map(|(path, fmt)| {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("failed to open log file");
        let base = tracing_fmt::layer()
            .with_writer(move || file.try_clone().unwrap())
            .with_ansi(false);
        match fmt.as_deref() {
            Some("json") => base.json().boxed(),
            _ => base.boxed(),
        }
    });
    let registry = registry.with(file_layer);
    Box::new(registry)
}

pub fn init(
    format: LogFormat,
    verbose: u8,
    info: &[InfoFlag],
    debug: &[DebugFlag],
    quiet: bool,
    log_file: Option<(PathBuf, Option<String>)>,
    syslog: bool,
    journald: bool,
) {
    subscriber(
        format, verbose, info, debug, quiet, log_file, syslog, journald,
    )
    .init();
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

pub fn render_out_format(
    format: &str,
    name: &Path,
    link: Option<&Path>,
    itemized: Option<&str>,
) -> String {
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(n) = chars.next() {
                match n {
                    'n' => out.push_str(&name.to_string_lossy()),
                    'L' => {
                        if let Some(l) = link {
                            out.push_str(" -> ");
                            out.push_str(&l.to_string_lossy());
                        }
                    }
                    'i' => {
                        if let Some(i) = itemized {
                            out.push_str(i);
                        }
                    }
                    'o' => {
                        if let Some(i) = itemized {
                            let op = match i.chars().next().unwrap_or(' ') {
                                '>' => "send",
                                '<' => "recv",
                                '*' => "del.",
                                _ => "",
                            };
                            out.push_str(op);
                        }
                    }
                    '%' => out.push('%'),
                    other => {
                        out.push('%');
                        out.push(other);
                    }
                }
            } else {
                out.push('%');
            }
        } else {
            out.push(c);
        }
    }
    out
}
