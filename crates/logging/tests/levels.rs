// crates/logging/tests/levels.rs
use logging::{subscriber, DebugFlag, InfoFlag, LogFormat, SubscriberConfig};
use tracing::subscriber::with_default;
use tracing::Level;

#[test]
fn info_not_emitted_by_default() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(0)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    let sub = subscriber(cfg).unwrap();
    with_default(sub, || {
        assert!(!tracing::enabled!(Level::INFO));
    });
}

#[test]
fn verbose_enables_info() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(1)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    let sub = subscriber(cfg).unwrap();
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}

#[test]
fn debug_enables_debug() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(0)
        .info(&[] as &[InfoFlag])
        .debug([DebugFlag::Flist])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    let sub = subscriber(cfg).unwrap();
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
    });
}

#[test]
fn debug_with_two_v() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(2)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    let sub = subscriber(cfg).unwrap();
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
    });
}

#[test]
fn info_flag_enables_info() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Text)
        .verbose(0)
        .info([InfoFlag::Progress])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    let sub = subscriber(cfg).unwrap();
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}

#[test]
fn json_verbose_enables_info() {
    let cfg = SubscriberConfig::builder()
        .format(LogFormat::Json)
        .verbose(1)
        .info(&[] as &[InfoFlag])
        .debug(&[] as &[DebugFlag])
        .quiet(false)
        .log_file(None)
        .syslog(false)
        .journald(false)
        .colored(true)
        .timestamps(false)
        .build();
    let sub = subscriber(cfg).unwrap();
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}
