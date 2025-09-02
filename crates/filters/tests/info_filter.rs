// crates/filters/tests/info_filter.rs
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use filters::{parse, Matcher};
use logging::InfoFlag;
use std::collections::HashSet;
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

fn m(input: &str) -> Matcher {
    let mut v = HashSet::new();
    Matcher::new(parse(input, &mut v, 0).unwrap())
}

#[test]
fn logs_match_and_rule_count() {
    let matcher = m("+ foo\n- *\n");

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
}
