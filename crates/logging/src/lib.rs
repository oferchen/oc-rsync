// crates/logging/src/lib.rs
#![allow(clippy::too_many_arguments)]
use std::fmt;
use std::fs::OpenOptions;
use std::io;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(all(unix, any(feature = "syslog", feature = "journald")))]
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use time::{macros::format_description, OffsetDateTime};
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    fmt as tracing_fmt,
    fmt::MakeWriter,
    layer::{Context, Layer, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter,
};

mod formatter;
pub use formatter::RsyncFormatter;

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
    Filter,
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
            InfoFlag::Filter => "filter",
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
            InfoFlag::Filter => "info::filter",
        }
    }
}

impl From<&InfoFlag> for InfoFlag {
    fn from(flag: &InfoFlag) -> Self {
        *flag
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Default)]
#[clap(rename_all = "kebab-case")]
pub enum StderrMode {
    #[clap(alias = "e")]
    #[default]
    Errors,
    #[clap(alias = "a")]
    All,
    #[clap(alias = "c")]
    Client,
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

impl From<&DebugFlag> for DebugFlag {
    fn from(flag: &DebugFlag) -> Self {
        *flag
    }
}

#[derive(Clone, Debug)]
pub struct SubscriberConfig {
    pub format: LogFormat,
    pub verbose: u8,
    pub info: Vec<InfoFlag>,
    pub debug: Vec<DebugFlag>,
    pub quiet: bool,
    pub stderr: StderrMode,
    pub log_file: Option<(PathBuf, Option<String>)>,
    pub syslog: bool,
    pub journald: bool,
    pub colored: bool,
    pub timestamps: bool,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Text,
            verbose: 0,
            info: Vec::new(),
            debug: Vec::new(),
            quiet: false,
            stderr: StderrMode::Errors,
            log_file: None,
            syslog: false,
            journald: false,
            colored: true,
            timestamps: false,
        }
    }
}

#[derive(Default)]
pub struct SubscriberConfigBuilder {
    cfg: SubscriberConfig,
}

impl SubscriberConfig {
    pub fn builder() -> SubscriberConfigBuilder {
        SubscriberConfigBuilder::default()
    }
}

impl SubscriberConfigBuilder {
    pub fn format(mut self, format: LogFormat) -> Self {
        self.cfg.format = format;
        self
    }

    pub fn verbose(mut self, verbose: u8) -> Self {
        self.cfg.verbose = verbose;
        self
    }

    pub fn info<I>(mut self, info: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<InfoFlag>,
    {
        self.cfg.info = info.into_iter().map(Into::into).collect();
        self
    }

    pub fn debug<I>(mut self, debug: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<DebugFlag>,
    {
        self.cfg.debug = debug.into_iter().map(Into::into).collect();
        self
    }

    pub fn quiet(mut self, quiet: bool) -> Self {
        self.cfg.quiet = quiet;
        self
    }

    pub fn stderr(mut self, stderr: StderrMode) -> Self {
        self.cfg.stderr = stderr;
        self
    }

    pub fn log_file(mut self, log_file: Option<(PathBuf, Option<String>)>) -> Self {
        self.cfg.log_file = log_file;
        self
    }

    pub fn syslog(mut self, syslog: bool) -> Self {
        self.cfg.syslog = syslog;
        self
    }

    pub fn journald(mut self, journald: bool) -> Self {
        self.cfg.journald = journald;
        self
    }

    pub fn colored(mut self, colored: bool) -> Self {
        self.cfg.colored = colored;
        self
    }

    pub fn timestamps(mut self, timestamps: bool) -> Self {
        self.cfg.timestamps = timestamps;
        self
    }

    pub fn build(self) -> SubscriberConfig {
        self.cfg
    }
}

#[cfg(all(unix, any(feature = "syslog", feature = "journald")))]
struct MessageVisitor {
    msg: String,
}

#[cfg(all(unix, any(feature = "syslog", feature = "journald")))]
impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(value);
        } else {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(field.name());
            self.msg.push('=');
            self.msg.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(&format!("{value:?}"));
        } else {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(field.name());
            self.msg.push('=');
            self.msg.push_str(&format!("{value:?}"));
        }
    }
}

#[cfg(all(unix, feature = "syslog"))]
struct SyslogLayer {
    sock: UnixDatagram,
}

#[cfg(all(unix, feature = "syslog"))]
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

#[cfg(all(unix, feature = "syslog"))]
fn syslog_severity(level: Level) -> u8 {
    match level {
        Level::ERROR => 3,
        Level::WARN => 4,
        Level::INFO => 6,
        Level::DEBUG | Level::TRACE => 7,
    }
}

#[cfg(all(unix, feature = "syslog"))]
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
        let pid = std::process::id();
        let data = format!("<{pri}>rsync[{pid}]: {}", v.msg);
        let _ = self.sock.send(data.as_bytes());
    }
}

#[cfg(all(unix, feature = "journald"))]
struct JournaldLayer {
    sock: UnixDatagram,
}

#[cfg(all(unix, feature = "journald"))]
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

#[cfg(all(unix, feature = "journald"))]
fn journald_priority(level: Level) -> u8 {
    match level {
        Level::ERROR => 3,
        Level::WARN => 4,
        Level::INFO => 6,
        Level::DEBUG | Level::TRACE => 7,
    }
}

#[cfg(all(unix, feature = "journald"))]
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
        let data = format!(
            "PRIORITY={prio}\nSYSLOG_IDENTIFIER=rsync\nMESSAGE={}\n",
            v.msg
        );
        let _ = self.sock.send(data.as_bytes());
    }
}

#[derive(Clone, Copy)]
struct LogWriter {
    mode: StderrMode,
}

impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = Box<dyn io::Write + Send + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        match self.mode {
            StderrMode::All => Box::new(io::stderr()),
            _ => Box::new(io::stdout()),
        }
    }

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        match self.mode {
            StderrMode::All => Box::new(io::stderr()),
            StderrMode::Client => Box::new(io::stdout()),
            StderrMode::Errors => {
                if meta.level() == &Level::ERROR {
                    Box::new(io::stderr())
                } else {
                    Box::new(io::stdout())
                }
            }
        }
    }
}

pub fn subscriber(cfg: SubscriberConfig) -> Box<dyn tracing::Subscriber + Send + Sync> {
    let SubscriberConfig {
        format,
        verbose,
        info,
        debug,
        quiet,
        stderr,
        log_file,
        syslog,
        journald,
        colored,
        timestamps,
    } = cfg;
    let mut level = if quiet {
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
    if !quiet {
        if !debug.is_empty() && level < LevelFilter::DEBUG {
            level = LevelFilter::DEBUG;
        } else if !info.is_empty() && level < LevelFilter::INFO {
            level = LevelFilter::INFO;
        }
    }
    let mut filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    if !quiet {
        for flag in &info {
            let directive: tracing_subscriber::filter::Directive =
                format!("{}=info", flag.target()).parse().unwrap();
            filter = filter.add_directive(directive);
        }
        for flag in &debug {
            let directive: tracing_subscriber::filter::Directive =
                format!("{}=trace", flag.target()).parse().unwrap();
            filter = filter.add_directive(directive);
        }
    }

    let writer = LogWriter { mode: stderr };
    let base = tracing_fmt::layer()
        .with_writer(writer)
        .with_target(false)
        .with_level(false);
    let base = if colored { base } else { base.with_ansi(false) };
    let fmt_layer = if timestamps {
        match format {
            LogFormat::Json => base.json().boxed(),
            LogFormat::Text => base.event_format(RsyncFormatter::new(None)).boxed(),
        }
    } else {
        let base = base.without_time();
        match format {
            LogFormat::Json => base.json().boxed(),
            LogFormat::Text => base.event_format(RsyncFormatter::new(None)).boxed(),
        }
    };

    #[cfg(all(unix, feature = "syslog"))]
    let syslog_layer = if syslog {
        SyslogLayer::new().ok()
    } else {
        None
    };
    #[cfg(not(all(unix, feature = "syslog")))]
    let syslog_layer: Option<tracing_subscriber::layer::Identity> = {
        let _ = syslog;
        None
    };
    #[cfg(all(unix, feature = "journald"))]
    let journald_layer = if journald {
        JournaldLayer::new().ok()
    } else {
        None
    };
    #[cfg(not(all(unix, feature = "journald")))]
    let journald_layer: Option<tracing_subscriber::layer::Identity> = {
        let _ = journald;
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
        let _ = (syslog, journald, syslog_layer, journald_layer);
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
            Some(spec) => base
                .event_format(RsyncFormatter::new(Some(spec.to_string())))
                .boxed(),
            None => base.event_format(RsyncFormatter::new(None)).boxed(),
        }
    });
    let registry = registry.with(file_layer);
    Box::new(registry)
}

pub fn init(cfg: SubscriberConfig) {
    subscriber(cfg).init();
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 9] = ["", "K", "M", "G", "T", "P", "E", "Z", "Y"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        size.trunc().to_string()
    } else {
        format!("{:.2}{}", size, UNITS[unit])
    }
}

pub fn progress_formatter(bytes: u64, human_readable: bool) -> String {
    if human_readable {
        human_bytes(bytes)
    } else {
        let mut n = bytes;
        let mut parts = Vec::new();
        while n >= 1000 {
            parts.push(format!("{:03}", n % 1000));
            n /= 1000;
        }
        parts.push(n.to_string());
        parts.reverse();
        parts.join(",")
    }
}

pub fn rate_formatter(bytes_per_sec: f64) -> String {
    let mut rate = bytes_per_sec / 1024.0;
    let mut units = "KB/s";
    if rate > 1024.0 {
        rate /= 1024.0;
        units = "MB/s";
        if rate > 1024.0 {
            rate /= 1024.0;
            units = "GB/s";
        }
    }
    format!("{:>7.2}{}", rate, units)
}

fn escape_bytes(bytes: &[u8], eight_bit_output: bool) -> String {
    let mut out = Vec::new();
    for &b in bytes {
        if (b < 0x20 && b != b'\t') || b == 0x7f || (!eight_bit_output && b >= 0x80) {
            out.extend_from_slice(format!("\\#{:03o}", b).as_bytes());
        } else {
            out.push(b);
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn escape_path(path: &Path, eight_bit_output: bool) -> String {
    #[cfg(unix)]
    {
        escape_bytes(path.as_os_str().as_bytes(), eight_bit_output)
    }
    #[cfg(not(unix))]
    {
        escape_bytes(path.to_string_lossy().as_bytes(), eight_bit_output)
    }
}

pub fn parse_escapes(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('a') => out.push('\x07'),
                Some('b') => out.push('\x08'),
                Some('e') => out.push('\x1b'),
                Some('f') => out.push('\x0c'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('v') => out.push('\x0b'),
                Some('x') => {
                    let mut val = 0u32;
                    let mut digits = 0;
                    for _ in 0..2 {
                        if let Some(peek) = chars.peek().copied() {
                            if let Some(digit) = peek.to_digit(16) {
                                val = (val << 4) + digit;
                                chars.next();
                                digits += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if digits == 0 {
                        tracing::warn!("invalid hex escape sequence");
                        out.push('x');
                    } else if let Some(ch) = char::from_u32(val) {
                        out.push(ch);
                    } else {
                        tracing::warn!("invalid hex escape value: {val}");
                    }
                }
                Some('\\') => out.push('\\'),
                Some(c @ '0'..='7') => {
                    let mut val = match c.to_digit(8) {
                        Some(d) => d,
                        None => {
                            tracing::warn!("invalid octal escape sequence");
                            out.push(c);
                            continue;
                        }
                    };
                    for _ in 0..2 {
                        if let Some(peek) = chars.peek().copied() {
                            if ('0'..='7').contains(&peek) {
                                if let Some(digit) = peek.to_digit(8) {
                                    val = (val << 3) + digit;
                                    chars.next();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    if let Some(ch) = char::from_u32(val) {
                        out.push(ch);
                    } else {
                        tracing::warn!("invalid octal escape value: {val}");
                    }
                }
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn render_out_format(
    format: &str,
    name: &Path,
    link: Option<&Path>,
    itemized: Option<&str>,
    eight_bit_output: bool,
) -> String {
    let fmt = parse_escapes(format);
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(n) = chars.next() {
                match n {
                    'n' | 'f' => out.push_str(&escape_path(name, eight_bit_output)),
                    'L' => {
                        if let Some(l) = link {
                            out.push_str(" -> ");
                            out.push_str(&escape_path(l, eight_bit_output));
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
                    'p' => out.push_str(&std::process::id().to_string()),
                    't' => {
                        let now = OffsetDateTime::now_local()
                            .unwrap_or_else(|_| OffsetDateTime::now_utc());
                        let fmt =
                            format_description!("[year]/[month]/[day] [hour]:[minute]:[second]");
                        out.push_str(&now.format(&fmt).unwrap());
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
