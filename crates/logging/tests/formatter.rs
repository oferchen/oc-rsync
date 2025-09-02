// crates/logging/tests/formatter.rs
use logging::RsyncFormatter;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, registry};

#[derive(Clone, Default)]
struct VecWriter(Arc<Mutex<Vec<u8>>>);

struct VecWriterGuard(Arc<Mutex<Vec<u8>>>);

impl Write for VecWriterGuard {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> fmt::writer::MakeWriter<'a> for VecWriter {
    type Writer = VecWriterGuard;
    fn make_writer(&'a self) -> Self::Writer {
        VecWriterGuard(self.0.clone())
    }
}

#[test]
fn respects_columns_env_var() {
    std::env::set_var("COLUMNS", "40");
    let writer = VecWriter::default();
    let layer = fmt::layer()
        .with_target(false)
        .with_level(false)
        .without_time()
        .event_format(RsyncFormatter)
        .with_ansi(false)
        .with_writer(writer.clone());
    let subscriber = registry().with(layer);
    tracing::subscriber::with_default(subscriber, || {
        info!(target: "test", "this is a very long line that should wrap around to the next line when the terminal width is small");
    });
    let out = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    let expected = include_str!("../../../tests/golden/logging/wrap_40.txt");
    assert_eq!(out, expected);
    std::env::remove_var("COLUMNS");
}
