// crates/logging/src/sink.rs

use crate::flags::StderrMode;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use tracing::{Level, Metadata};
use tracing_subscriber::fmt::MakeWriter;

pub trait ProgressSink: Send + Sync {
    fn start_file(&self, path: &Path, total: u64, written: u64);
    fn update(&self, written: u64);
    fn finish_file(&self);
    fn progress(&self, line: &str);
}

#[derive(Debug, Default)]
pub struct NopProgressSink;

impl ProgressSink for NopProgressSink {
    fn start_file(&self, _path: &Path, _total: u64, _written: u64) {}
    fn update(&self, _written: u64) {}
    fn finish_file(&self) {}
    fn progress(&self, _line: &str) {}
}

impl<F> ProgressSink for F
where
    F: Fn(&str) + Send + Sync,
{
    fn start_file(&self, _path: &Path, _total: u64, _written: u64) {}
    fn update(&self, _written: u64) {}
    fn finish_file(&self) {}
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

impl Write for FileWriterHandle {
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
