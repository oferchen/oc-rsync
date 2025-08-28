use criterion::{criterion_group, criterion_main, Criterion};
use engine::{compute_delta, ChecksumConfigBuilder};
use std::io::Cursor;

fn bench_large_delta(c: &mut Criterion) {
    let cfg = ChecksumConfigBuilder::new().build();
    let block_size = 1024;
    let window = 64;
    let data = vec![0u8; block_size * 1024]; // 1 MiB
    c.bench_function("compute_delta_large_file", |b| {
        b.iter(|| {
            let mut basis = Cursor::new(data.clone());
            let mut target = Cursor::new(data.clone());
            compute_delta(&cfg, &mut basis, &mut target, block_size, window).unwrap();
        });
    });
}

criterion_group!(benches, bench_large_delta);
criterion_main!(benches);
