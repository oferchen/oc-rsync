// crates/logging/src/sink.rs
use std::path::Path;

/// A sink for receiving progress events during file transfers.
///
/// Implementations can forward progress information to arbitrary
/// destinations such as a user interface or test harness.
pub trait ProgressSink: Send + Sync {
    /// Notify that a new file transfer is starting.
    fn start_file(&self, path: &Path, total: u64, written: u64);

    /// Notify that `written` bytes have been transferred so far.
    fn update(&self, written: u64);

    /// Notify that the current file transfer has finished.
    fn finish_file(&self);
}

/// A no-op progress sink used when progress reporting is disabled.
#[derive(Debug, Default)]
pub struct NopProgressSink;

impl ProgressSink for NopProgressSink {
    fn start_file(&self, _path: &Path, _total: u64, _written: u64) {}
    fn update(&self, _written: u64) {}
    fn finish_file(&self) {}
}
