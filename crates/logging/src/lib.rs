// crates/logging/src/lib.rs

use std::fmt;
use std::fs::OpenOptions;
use std::io;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(all(unix, any(feature = "syslog", feature = "journald")))]
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use time::{OffsetDateTime, macros::format_description};
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    EnvFilter, fmt as tracing_fmt,
    layer::{Context, Layer, SubscriberExt},
    util::SubscriberInitExt,
};

mod flags;
mod formatter;
mod sink;

pub use flags::{
    DebugFlag, InfoFlag, LogFormat, StderrMode, SubscriberConfig, SubscriberConfigBuilder,
};
pub use formatter::RsyncFormatter;
pub use sink::ProgressSink;

use crate::sink::{FileWriter, LogWriter};

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

pub fn subscriber(cfg: SubscriberConfig) -> io::Result<Box<dyn tracing::Subscriber + Send + Sync>> {
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

    let file_layer = if let Some((path, fmt)) = log_file {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let _clone_check = file.try_clone()?;
        let base = tracing_fmt::layer()
            .with_writer(FileWriter { file })
            .with_ansi(false);
        let layer = match fmt.as_deref() {
            Some("json") => base.json().boxed(),
            Some(spec) => base
                .event_format(RsyncFormatter::new(Some(spec.to_string())))
                .boxed(),
            None => base.event_format(RsyncFormatter::new(None)).boxed(),
        };
        Some(layer)
    } else {
        None
    };
    let registry = registry.with(file_layer);
    Ok(Box::new(registry))
}

pub fn init(cfg: SubscriberConfig) -> io::Result<()> {
    subscriber(cfg)?.init();
    Ok(())
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
    let mut out = String::new();
    for &b in bytes {
        if (b < 0x20 && b != b'\t') || (!eight_bit_output && b > 0x7e) {
            out.push_str(&format!("\\#{:03o}", b));
        } else {
            out.push(char::from(b));
        }
    }
    out
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

pub struct OutFormatOptions<'a> {
    name: &'a Path,
    link: Option<&'a Path>,
    itemized: Option<&'a str>,
    eight_bit_output: bool,
}

impl<'a> OutFormatOptions<'a> {
    pub fn new(name: &'a Path) -> Self {
        Self {
            name,
            link: None,
            itemized: None,
            eight_bit_output: false,
        }
    }

    pub fn link(mut self, link: Option<&'a Path>) -> Self {
        self.link = link;
        self
    }

    pub fn itemized(mut self, itemized: Option<&'a str>) -> Self {
        self.itemized = itemized;
        self
    }

    pub fn eight_bit_output(mut self, eight_bit_output: bool) -> Self {
        self.eight_bit_output = eight_bit_output;
        self
    }
}

pub fn render_out_format(format: &str, opts: &OutFormatOptions<'_>) -> String {
    let fmt = parse_escapes(format);
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(n) = chars.next() {
                match n {
                    'n' | 'f' => out.push_str(&escape_path(opts.name, opts.eight_bit_output)),
                    'L' => {
                        if let Some(l) = opts.link {
                            out.push_str(" -> ");
                            out.push_str(&escape_path(l, opts.eight_bit_output));
                        }
                    }
                    'i' => {
                        if let Some(i) = opts.itemized {
                            out.push_str(i);
                        }
                    }
                    'o' => {
                        if let Some(i) = opts.itemized {
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
