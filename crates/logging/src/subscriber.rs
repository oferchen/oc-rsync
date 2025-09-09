// crates/logging/src/subscriber.rs
#![allow(missing_docs)]

use crate::flags::{LogFormat, SubscriberConfig};
use crate::formatter::RsyncFormatter;
use crate::sink::{FileWriter, LogWriter};
use std::fmt;
use std::fs::OpenOptions;
use std::io;
use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{
    EnvFilter, fmt as tracing_fmt,
    layer::{Context, Layer, SubscriberExt},
    util::SubscriberInitExt,
};

#[cfg(all(unix, any(feature = "syslog", feature = "journald")))]
use std::os::unix::net::UnixDatagram;
#[cfg(all(unix, any(feature = "syslog", feature = "journald")))]
use std::path::PathBuf;

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
    fn new() -> io::Result<Self> {
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
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
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
    fn new() -> io::Result<Self> {
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
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
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

/// Build a [`tracing`] subscriber configured for rsync logging semantics.
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

/// Initialise the global [`tracing`] subscriber.
pub fn init(cfg: SubscriberConfig) -> io::Result<()> {
    subscriber(cfg)?.init();
    Ok(())
}
