// crates/engine/benches/large_files.rs
use checksums::ChecksumConfigBuilder;
use criterion::{criterion_group, criterion_main, Criterion};
use engine::{compute_delta, SyncOptions};
use std::io::{Cursor, Read, Seek, SeekFrom};

fn bench_large_delta(c: &mut Criterion) {
    let cfg = ChecksumConfigBuilder::new().build();
    let block_size = 1024;
    let window = 64;
    let data = vec![0u8; block_size * 1024];
    c.bench_function("compute_delta_large_file", |b| {
        b.iter(|| {
            let mut basis = Cursor::new(data.clone());
            let mut target = Cursor::new(data.clone());
            for op in compute_delta(
                &cfg,
                &mut basis,
                &mut target,
                block_size,
                window,
                &SyncOptions::default(),
            )
            .unwrap()
            {
                op.unwrap();
            }
        });
    });
}

struct ZeroReader {
    pos: u64,
    len: u64,
}

impl ZeroReader {
    fn new(len: u64) -> Self {
        Self { pos: 0, len }
    }
}

impl Read for ZeroReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.len {
            return Ok(0);
        }
        let remain = (self.len - self.pos) as usize;
        let n = remain.min(buf.len());
        for b in &mut buf[..n] {
            *b = 0;
        }
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for ZeroReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new = match pos {
            SeekFrom::Start(o) => o as i64,
            SeekFrom::End(o) => self.len as i64 + o,
            SeekFrom::Current(o) => self.pos as i64 + o,
        };
        self.pos = new as u64;
        Ok(self.pos)
    }
}

fn bench_streaming_delta(c: &mut Criterion) {
    let cfg = ChecksumConfigBuilder::new().build();
    let block_size = 1024;
    let window = 64;
    let len = 2 * 1024 * 1024 * 1024u64;
    c.bench_function("stream_delta_2gb_zero", |b| {
        b.iter(|| {
            let mut basis = ZeroReader::new(len);
            let mut target = ZeroReader::new(len);
            for op in compute_delta(
                &cfg,
                &mut basis,
                &mut target,
                block_size,
                window,
                &SyncOptions::default(),
            )
            .unwrap()
            {
                op.unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_large_delta, bench_streaming_delta);
criterion_main!(benches);
