// crates/engine/benches/rolling.rs
use checksums::rolling_checksum_seeded;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_rolling(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024];
    c.bench_function("rolling_checksum_1mb", |b| {
        b.iter(|| {
            criterion::black_box(rolling_checksum_seeded(&data, 0));
        });
    });
}

criterion_group!(benches, bench_rolling);
criterion_main!(benches);
