// crates/logging/tests/levels.rs
use logging::{subscriber, DebugFlag, InfoFlag, LogFormat};
use tracing::subscriber::with_default;
use tracing::Level;

#[test]
fn info_not_emitted_by_default() {
    let sub = subscriber(LogFormat::Text, 0, &[], &[], false, None, false, false);
    with_default(sub, || {
        assert!(!tracing::enabled!(Level::INFO));
    });
}

#[test]
fn verbose_enables_info() {
    let sub = subscriber(LogFormat::Text, 1, &[], &[], false, None, false, false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}

#[test]
fn debug_enables_debug() {
    let sub = subscriber(
        LogFormat::Text,
        0,
        &[],
        &[DebugFlag::Flist],
        false,
        None,
        false,
        false,
    );
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
    });
}

#[test]
fn debug_with_two_v() {
    let sub = subscriber(LogFormat::Text, 2, &[], &[], false, None, false, false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
    });
}

#[test]
fn info_flag_enables_info() {
    let sub = subscriber(
        LogFormat::Text,
        0,
        &[InfoFlag::Progress],
        &[],
        false,
        None,
        false,
        false,
    );
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}

#[test]
fn json_verbose_enables_info() {
    let sub = subscriber(LogFormat::Json, 1, &[], &[], false, None, false, false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}
