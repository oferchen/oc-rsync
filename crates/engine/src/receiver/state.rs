// crates/engine/src/receiver/state.rs
use std::path::{Path, PathBuf};
use std::sync::Arc;

use compress::Codec;
use filters::Matcher;
use logging::{NopObserver, Observer};

use crate::SyncOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverState {
    Idle,
    Applying,
    Finished,
}

pub struct Receiver {
    pub(super) state: ReceiverState,
    pub(super) codec: Option<Codec>,
    pub(super) opts: SyncOptions,
    pub(crate) matcher: Matcher,
    pub(super) delayed: Vec<(PathBuf, PathBuf, PathBuf)>,
    #[cfg(unix)]
    pub(super) link_map: meta::HardLinks,
    pub(super) progress_sink: Arc<dyn Observer>,
}

impl Default for Receiver {
    fn default() -> Self {
        Self::new(None, SyncOptions::default())
    }
}

impl Receiver {
    pub fn new(codec: Option<Codec>, opts: SyncOptions) -> Self {
        Self {
            state: ReceiverState::Idle,
            codec,
            opts,
            matcher: Matcher::default(),
            delayed: Vec::new(),
            #[cfg(unix)]
            link_map: meta::HardLinks::default(),
            progress_sink: Arc::new(NopObserver),
        }
    }

    pub fn set_progress_sink(&mut self, sink: Arc<dyn Observer>) {
        self.progress_sink = sink;
    }

    #[cfg(unix)]
    pub fn register_hard_link(&mut self, id: u64, path: &Path) -> bool {
        self.link_map.register(id, path)
    }
}
