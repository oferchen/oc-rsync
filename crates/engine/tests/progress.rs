// crates/engine/tests/progress.rs
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use engine::delta::Op;
use engine::{Receiver, SyncOptions};
use logging::ProgressSink;
use tempfile::tempdir;

#[derive(Default)]
struct MockSink {
    events: Mutex<Vec<String>>,
}

impl ProgressSink for MockSink {
    fn start_file(&self, _path: &Path, total: u64, written: u64) {
        self.events
            .lock()
            .unwrap()
            .push(format!("start:{total}:{written}"));
    }

    fn update(&self, written: u64) {
        self.events
            .lock()
            .unwrap()
            .push(format!("update:{written}"));
    }

    fn finish_file(&self) {
        self.events.lock().unwrap().push("finish".into());
    }
}

#[test]
fn receiver_emits_progress_events() {
    let dir = tempdir().unwrap();
    let src_path = dir.path().join("src");
    File::create(&src_path)
        .unwrap()
        .write_all(b"basis")
        .unwrap();
    let dest_path = dir.path().join("dest");

    let sink = Arc::new(MockSink::default());
    let mut opts = SyncOptions::default();
    opts.progress = true;
    opts.quiet = true;
    let mut recv = Receiver::new(None, opts);
    recv.set_progress_sink(sink.clone());

    let delta = vec![Ok(Op::Data(b"abcd".to_vec()))];
    recv.apply(&src_path, &dest_path, Path::new(""), delta)
        .unwrap();

    let events = sink.events.lock().unwrap().clone();
    assert_eq!(events, vec!["start:4:0", "update:4", "finish"]);
}
