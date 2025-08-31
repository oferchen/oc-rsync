use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt,
    layer::{Layer as _, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter,
};

#[derive(Clone, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

pub fn subscriber(
    format: LogFormat,
    verbose: u8,
    info: bool,
    debug: bool,
) -> impl tracing::Subscriber + Send + Sync {
    let level = if debug || verbose > 1 {
        LevelFilter::DEBUG
    } else if info || verbose > 0 {
        LevelFilter::INFO
    } else {
        LevelFilter::WARN
    };
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    let fmt_layer = fmt::layer();
    let fmt_layer = match format {
        LogFormat::Json => fmt_layer.json().boxed(),
        LogFormat::Text => fmt_layer.boxed(),
    };

    tracing_subscriber::registry().with(filter).with(fmt_layer)
}

pub fn init(format: LogFormat, verbose: u8, info: bool, debug: bool) {
    subscriber(format, verbose, info, debug).init();
}

/// Format a byte count using human-readable units.
///
/// Values are formatted using binary prefixes (KiB, MiB, etc.) and
/// fixed to two decimal places when a prefix is used. For values
/// smaller than 1 KiB, the output is expressed directly in bytes.
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

/// Return a formatted string for progress output.
///
/// When `human_readable` is `true`, the byte count is converted using
/// [`human_bytes`]; otherwise the raw byte count followed by the word
/// `bytes` is returned. This mirrors the behaviour of upstream rsync's
/// progress output.
pub fn progress_formatter(bytes: u64, human_readable: bool) -> String {
    if human_readable {
        human_bytes(bytes)
    } else {
        format!("{} bytes", bytes)
    }
}
