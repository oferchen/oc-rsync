// crates/logging/tests/info_flags.rs
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use clap::ValueEnum;
use logging::InfoFlag;
use tracing::level_filters::LevelFilter;
use tracing::subscriber::with_default;
use tracing_subscriber::{
    fmt::{self, writer::MakeWriter},
    layer::SubscriberExt,
    EnvFilter,
};

#[derive(Clone, Default)]
struct VecWriter(Arc<Mutex<Vec<u8>>>);

struct VecWriterGuard<'a>(Arc<Mutex<Vec<u8>>>);

impl<'a> Write for VecWriterGuard<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for VecWriter {
    type Writer = VecWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        VecWriterGuard(self.0.clone())
    }
}

impl VecWriter {
    fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}

#[test]
fn each_info_flag_emits_output() {
    for flag in InfoFlag::value_variants() {
        let writer = VecWriter::default();
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::WARN.into())
            .from_env_lossy();
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(writer.clone()));
        with_default(subscriber, || {
            tracing::info!(target: flag.target(), "{}", flag.as_str());
        });
        assert!(writer.is_empty(), "{} emitted without flag", flag.as_str());

        let writer = VecWriter::default();
        let mut filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::WARN.into())
            .from_env_lossy();
        filter = filter.add_directive(format!("{}=info", flag.target()).parse().unwrap());
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(writer.clone()));
        with_default(subscriber, || {
            tracing::info!(target: flag.target(), "{}", flag.as_str());
        });
        assert!(!writer.is_empty(), "{} did not emit output", flag.as_str());
    }
}
