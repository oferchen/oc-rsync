// crates/engine/benches/compress.rs
use compress::Compressor;
#[cfg(feature = "lz4")]
use compress::Lz4;
#[cfg(feature = "zstd")]
use compress::Zstd;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_compress(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024];
    #[cfg(feature = "zstd")]
    {
        let zstd = Zstd::default();
        c.bench_function("zstd_compress_1mb", |b| {
            b.iter(|| {
                zstd.compress(&data).unwrap();
            });
        });
    }
    #[cfg(feature = "lz4")]
    {
        let lz4 = Lz4;
        c.bench_function("lz4_compress_1mb", |b| {
            b.iter(|| {
                lz4.compress(&data).unwrap();
            });
        });
    }
}

criterion_group!(benches, bench_compress);
criterion_main!(benches);
