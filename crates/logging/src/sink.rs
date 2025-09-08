// crates/logging/src/sink.rs

use crate::flags::StderrMode;
use std::fs::File;
#[allow(unused_imports)]
use std::io::{self, Write};
use tracing::{Level, Metadata};
use tracing_subscriber::fmt::MakeWriter;

/// Trait for handling progress output independently of logging.
pub trait ProgressSink: Send + Sync {
    /// Handle a progress line.
    fn progress(&self, line: &str);
}

impl<F> ProgressSink for F
where
    F: Fn(&str) + Send + Sync,
{
    fn progress(&self, line: &str) {
        self(line);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LogWriter {
    pub(crate) mode: StderrMode,
}

impl<'a> MakeWriter<'a> for LogWriter {
    type Writer = Box<dyn io::Write + Send + 'a>;

    fn make_writer(&'a self) -> Self::Writer {
        match self.mode {
            StderrMode::All => Box::new(io::stderr()),
            _ => Box::new(io::stdout()),
        }
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        match self.mode {
            StderrMode::All => Box::new(io::stderr()),
            StderrMode::Client => Box::new(io::stdout()),
            StderrMode::Errors => {
                if meta.level() == &Level::ERROR {
                    Box::new(io::stderr())
                } else {
                    Box::new(io::stdout())
                }
            }
        }
    }
}

pub(crate) struct FileWriter {
    pub(crate) file: File,
}

pub(crate) struct FileWriterHandle(pub(crate) io::Result<File>);

impl io::Write for FileWriterHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.0 {
            Ok(f) => f.write(buf),
            Err(e) => Err(io::Error::new(e.kind(), e.to_string())),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.0 {
            Ok(f) => f.flush(),
            Err(e) => Err(io::Error::new(e.kind(), e.to_string())),
        }
    }
}

impl<'a> MakeWriter<'a> for FileWriter {
    type Writer = FileWriterHandle;

    fn make_writer(&'a self) -> Self::Writer {
        FileWriterHandle(self.file.try_clone())
    }
}
