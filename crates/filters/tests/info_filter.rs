// crates/filters/tests/info_filter.rs
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use filters::{parse_file, Matcher};
use logging::InfoFlag;
use std::collections::HashSet;
use tempfile::NamedTempFile;
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
#[test]
fn logs_match_and_rule_count() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "+ foo\n- *\n").unwrap();
    let mut v = HashSet::new();
    let rules = parse_file(tmp.path(), false, &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);

    let writer = VecWriter::default();
    let mut filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into())
        .from_env_lossy();
    filter = filter.add_directive(
        format!("{}=info", InfoFlag::Filter.target())
            .parse()
            .unwrap(),
    );
    let subscriber = tracing_subscriber::registry().with(filter).with(
        fmt::layer()
            .without_time()
            .with_ansi(false)
            .with_writer(writer.clone()),
    );
    with_default(subscriber, || {
        assert!(matcher.is_included("foo").unwrap());
    });

    let log = String::from_utf8(writer.0.lock().unwrap().clone()).unwrap();
    let line = log.lines().find(|l| l.contains("info::filter")).unwrap();
    assert!(line.contains("matched=true"));
    assert!(line.contains("rules=2"));
    assert!(line.contains("matches=1"));
    assert!(line.contains("misses=0"));
    assert!(line.contains(&format!("source={}", tmp.path().display())));
}
