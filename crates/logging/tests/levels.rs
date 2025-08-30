use logging::{subscriber, LogFormat};
use tracing::Level;
use tracing::subscriber::with_default;

#[test]
fn info_not_emitted_by_default() {
    let sub = subscriber(LogFormat::Text, 0, false, false);
    with_default(sub, || {
        assert!(!tracing::enabled!(Level::INFO));
    });
}

#[test]
fn verbose_enables_info() {
    let sub = subscriber(LogFormat::Text, 1, false, false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}

#[test]
fn debug_enables_debug() {
    let sub = subscriber(LogFormat::Text, 0, false, true);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
    });
}

#[test]
fn debug_with_two_v() {
    let sub = subscriber(LogFormat::Text, 2, false, false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
    });
}

#[test]
fn info_flag_enables_info() {
    let sub = subscriber(LogFormat::Text, 0, true, false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
    });
}
