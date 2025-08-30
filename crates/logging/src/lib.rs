use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt,
    layer::{Layer as _, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter,
};

/// Output format for log records.
#[derive(Clone, Copy)]
pub enum LogFormat {
    Text,
    Json,
}

/// Build a `tracing` subscriber with the requested configuration.
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

/// Initialize global logging.
pub fn init(format: LogFormat, verbose: u8, info: bool, debug: bool) {
    subscriber(format, verbose, info, debug).init();
}
