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

impl<'a> MakeWriter<'a> for VecWriter {
    type Writer = VecWriterGuard;

    fn make_writer(&'a self) -> Self::Writer {
        VecWriterGuard(self.0.clone())
    }
}

fn emit(flag: InfoFlag) {
    match flag {
        InfoFlag::Backup => tracing::info!(target: InfoFlag::Backup.target(), "backup"),
        InfoFlag::Copy => tracing::info!(target: InfoFlag::Copy.target(), "copy"),
        InfoFlag::Del => tracing::info!(target: InfoFlag::Del.target(), "del"),
        InfoFlag::Flist => tracing::info!(target: InfoFlag::Flist.target(), "flist"),
        InfoFlag::Flist2 => tracing::info!(target: InfoFlag::Flist2.target(), "flist2"),
        InfoFlag::Misc => tracing::info!(target: InfoFlag::Misc.target(), "misc"),
        InfoFlag::Misc2 => tracing::info!(target: InfoFlag::Misc2.target(), "misc2"),
        InfoFlag::Mount => tracing::info!(target: InfoFlag::Mount.target(), "mount"),
        InfoFlag::Name => tracing::info!(target: InfoFlag::Name.target(), "name"),
        InfoFlag::Name2 => tracing::info!(target: InfoFlag::Name2.target(), "name2"),
        InfoFlag::Nonreg => tracing::info!(target: InfoFlag::Nonreg.target(), "nonreg"),
        InfoFlag::Progress => tracing::info!(target: InfoFlag::Progress.target(), "progress"),
        InfoFlag::Progress2 => tracing::info!(target: InfoFlag::Progress2.target(), "progress2"),
        InfoFlag::Remove => tracing::info!(target: InfoFlag::Remove.target(), "remove"),
        InfoFlag::Skip => tracing::info!(target: InfoFlag::Skip.target(), "skip"),
        InfoFlag::Skip2 => tracing::info!(target: InfoFlag::Skip2.target(), "skip2"),
        InfoFlag::Stats => tracing::info!(target: InfoFlag::Stats.target(), "stats"),
        InfoFlag::Stats2 => tracing::info!(target: InfoFlag::Stats2.target(), "stats2"),
        InfoFlag::Stats3 => tracing::info!(target: InfoFlag::Stats3.target(), "stats3"),
        InfoFlag::Symsafe => tracing::info!(target: InfoFlag::Symsafe.target(), "symsafe"),
        InfoFlag::Filter => tracing::info!(target: InfoFlag::Filter.target(), "filter"),
    }
}

#[test]
fn each_info_flag_enables_when_specified() {
    for &flag in InfoFlag::value_variants() {
        let writer = VecWriter::default();
        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::WARN.into())
            .from_env_lossy();
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(writer.clone()));
        with_default(subscriber, || {
            emit(flag);
        });
        assert!(
            writer.0.lock().unwrap().is_empty(),
            "{} emitted without flag",
            flag.as_str()
        );

        let writer = VecWriter::default();
        let mut filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::WARN.into())
            .from_env_lossy();
        filter = filter.add_directive(format!("{}=info", flag.target()).parse().unwrap());
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().with_writer(writer.clone()));
        with_default(subscriber, || {
            emit(flag);
        });
        assert!(
            !writer.0.lock().unwrap().is_empty(),
            "{} did not emit output",
            flag.as_str()
        );
    }
}
